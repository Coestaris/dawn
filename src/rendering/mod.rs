use crate::components::imui::UICommand;
use crate::logging;
use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::{RenderingEvent, RenderingEventMask};
use crate::rendering::fullscreen::setup_fullscreen_system;
use crate::rendering::gbuffer::GBuffer;
use crate::rendering::passes::bounding_pass::BoundingPass;
use crate::rendering::passes::geometry_pass::GeometryPass;
use crate::rendering::passes::screen_pass::ScreenPass;
use crate::rendering::passes::ui_pass::UIPass;
use crate::systems::asset::FactoryBindings;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_graphics::construct_chain;
use dawn_graphics::gl::bindings;
use dawn_graphics::input::InputEvent;
use dawn_graphics::passes::chain::ChainCons;
use dawn_graphics::passes::chain::ChainNil;
use dawn_graphics::passes::events::RenderPassEvent;
use dawn_graphics::passes::pipeline::RenderPipeline;
use dawn_graphics::renderer::{Renderer, RendererBackendConfig};
use dawn_graphics::view::{
    PlatformSpecificViewConfig, ViewConfig, ViewCursor, ViewGeometry, ViewSynchronization,
};
use evenio::event::Receiver;
use evenio::prelude::*;
use glam::UVec2;
use std::ops::Deref;
use std::rc::Rc;
use triple_buffer::Output;

pub mod dispatcher;
pub mod event;
pub mod frustum;
pub mod fullscreen;
pub mod gbuffer;
pub mod passes;
pub mod primitive;

static WINDOW_SIZE: UVec2 = UVec2::new(1280, 720);

fn asset_events_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    dispatcher: Single<&RenderDispatcher>,
    sender: Sender<RenderPassEvent<RenderingEvent>>,
) {
    dispatcher.dispatch_assets(r.event, hub.0, sender);
}

fn input_events_handler(
    r: Receiver<InputEvent>,
    mut dispatcher: Single<&mut RenderDispatcher>,
    sender: Sender<RenderPassEvent<RenderingEvent>>,
) {
    dispatcher.dispatch_input(r.event, sender);
}

pub fn setup_rendering_system(
    world: &mut World,
    bindings: FactoryBindings,
    synchronization: Option<ViewSynchronization>,
    ui_stream: Output<Vec<UICommand>>,
) {
    let bi = logging::dawn_build_info();
    let view_config = ViewConfig {
        platform_specific: PlatformSpecificViewConfig {},
        title: format!("Dawn v{} - {}", bi.crate_info.version, bi.profile),
        geometry: ViewGeometry::Normal(WINDOW_SIZE),
        cursor: ViewCursor::Crosshair,
        synchronization,
    };

    let backend_config = RendererBackendConfig {
        shader_factory_binding: Some(bindings.shader),
        texture_factory_binding: Some(bindings.texture),
        mesh_factory_binding: Some(bindings.mesh),
        material_factory_binding: Some(bindings.material),
        font_factory_binding: Some(bindings.font),
    };

    let mut dispatcher = RenderDispatcher::new();
    let geometry_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::VIEW_UPDATED
            | RenderingEventMask::VIEWPORT_RESIZED
            | RenderingEventMask::PERSP_PROJECTION_UPDATED
            | RenderingEventMask::TOGGLE_WIREFRAME_MODE,
        "geometry_shader",
    );
    let bounding_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::VIEW_UPDATED
            | RenderingEventMask::VIEWPORT_RESIZED
            | RenderingEventMask::PERSP_PROJECTION_UPDATED
            | RenderingEventMask::TOGGLE_BOUNDING,
        "bounding_shader",
    );
    let ui_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::ORTHO_PROJECTION_UPDATED,
        "glyph_shader",
    );
    let screen_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::VIEWPORT_RESIZED,
        "screen_shader",
    );

    let renderer = Renderer::new_with_monitoring(view_config, backend_config, move |_| {
        // Setup OpenGL state
        unsafe {
            // Enable wireframe mode
            bindings::Enable(bindings::DEPTH_TEST);
            bindings::DepthFunc(bindings::LEQUAL);
            bindings::Enable(bindings::MULTISAMPLE);
            bindings::Hint(bindings::PERSPECTIVE_CORRECTION_HINT, bindings::NICEST);
            bindings::Enable(bindings::BLEND);
            bindings::BlendFunc(bindings::SRC_ALPHA, bindings::ONE_MINUS_SRC_ALPHA);
        }

        let gbuffer = Rc::new(GBuffer::new(WINDOW_SIZE));

        let geometry_pass = GeometryPass::new(geometry_id, gbuffer.clone());
        let bounding_pass = BoundingPass::new(bounding_id, gbuffer.clone());
        let ui_pass = UIPass::new(ui_id, ui_stream);
        let screen_pass = ScreenPass::new(screen_id, gbuffer.clone());
        Ok(RenderPipeline::new(construct_chain!(
            geometry_pass,
            screen_pass,
            bounding_pass,
            ui_pass,
        )))
    })
    .unwrap();
    renderer.attach_to_ecs(world);

    // Move dispatcher to the world
    let e = world.spawn();
    world.insert(e, dispatcher);

    world.add_handler(asset_events_handler);
    world.add_handler(input_events_handler);

    setup_fullscreen_system(world);
}
