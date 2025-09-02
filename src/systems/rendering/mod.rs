use crate::components::imui::UICommand;
use crate::logging;
use crate::systems::asset::FactoryBindings;
use crate::systems::rendering::aabb_pass::AABBPass;
use crate::systems::rendering::gbuffer::GBuffer;
use crate::systems::rendering::geometry_pass::GeometryPass;
use crate::systems::rendering::screen_pass::ScreenPass;
use crate::systems::rendering::ui_pass::UIPass;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::ir::texture::{IRPixelFormat, IRTextureType};
use dawn_assets::TypedAsset;
use dawn_ecs::events::ExitEvent;
use dawn_graphics::construct_chain;
use dawn_graphics::gl::font::Font;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use dawn_graphics::gl::raii::shader_program::ShaderProgram;
use dawn_graphics::gl::raii::texture::Texture;
use dawn_graphics::gl::{bindings, raii};
use dawn_graphics::input::{InputEvent, KeyCode};
use dawn_graphics::passes::chain::ChainCons;
use dawn_graphics::passes::chain::ChainNil;
use dawn_graphics::passes::events::{RenderPassEvent, RenderPassTargetId};
use dawn_graphics::passes::pipeline::RenderPipeline;
use dawn_graphics::renderer::{Renderer, RendererBackendConfig, ViewEvent};
use dawn_graphics::view::{
    PlatformSpecificViewConfig, ViewConfig, ViewCursor, ViewGeometry, ViewSynchronization,
};
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::prelude::World;
use glam::{Mat4, UVec2};
use log::info;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use triple_buffer::Output;

mod aabb_pass;
mod gbuffer;
mod geometry_pass;
mod screen_pass;
mod ui_pass;

const WINDOW_SIZE: (u32, u32) = (1280, 720);

#[derive(Debug, Clone)]
pub(crate) enum CustomPassEvent {
    DropAllAssets,
    UpdateShader(TypedAsset<ShaderProgram>),
    ToggleWireframeMode,
    ToggleAABB,
    UpdateView(Mat4),
    UpdateWindowSize(UVec2),
}

#[derive(Component)]
struct CurrentGeometry {
    is_fullscreen: bool,
}

#[derive(Component)]
#[component(immutable)]
pub struct RenderPassIDs {
    pub geometry: RenderPassTargetId,
    pub aabb: RenderPassTargetId,
    pub ui: RenderPassTargetId,
    pub screen: RenderPassTargetId,
}

fn map_shaders_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    ids: Single<&RenderPassIDs>,
    mut sender: Sender<(ExitEvent, RenderPassEvent<CustomPassEvent>)>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) => {
            let map = HashMap::from([
                ("geometry_shader", ids.geometry),
                ("glyph_shader", ids.ui),
                ("aabb_shader", ids.aabb),
                ("screen_shader", ids.screen),
            ]);

            if let Some(target) = map.get(id.as_str()) {
                let shader = hub.get_typed::<ShaderProgram>(id.clone()).unwrap();
                sender.send(RenderPassEvent::new(
                    *target,
                    CustomPassEvent::UpdateShader(shader),
                ));
            }
        }
        _ => {}
    }
}

fn viewport_resized_handler(
    ie: Receiver<InputEvent>,
    ids: Single<&RenderPassIDs>,
    mut sender: Sender<RenderPassEvent<CustomPassEvent>>,
) {
    match ie.event {
        InputEvent::Resize { width, height } => {
            info!("Viewport resized to {}x{}", width, height);
            let broadcast = [ids.geometry, ids.aabb, ids.ui, ids.screen];
            for id in broadcast.iter() {
                sender.send(RenderPassEvent::new(
                    *id,
                    CustomPassEvent::UpdateWindowSize(UVec2::new(*width as u32, *height as u32)),
                ));
            }
        }

        InputEvent::KeyPress(KeyCode::Function(3)) => {
            sender.send(RenderPassEvent::new(
                ids.geometry,
                CustomPassEvent::ToggleWireframeMode,
            ));
        }
        InputEvent::KeyPress(KeyCode::Function(4)) => {
            sender.send(RenderPassEvent::new(ids.aabb, CustomPassEvent::ToggleAABB));
        }
        _ => {}
    }
}

fn fullscreen_handler(
    ie: Receiver<InputEvent>,
    mut cg: Single<&mut CurrentGeometry>,
    mut sender: Sender<ViewEvent>,
) {
    match ie.event {
        InputEvent::KeyPress(KeyCode::Function(11)) => {
            cg.is_fullscreen = !cg.is_fullscreen;
            if cg.is_fullscreen {
                sender.send(ViewEvent::SetGeometry(ViewGeometry::BorderlessFullscreen));
            } else {
                sender.send(ViewEvent::SetGeometry(ViewGeometry::Normal(
                    WINDOW_SIZE.0,
                    WINDOW_SIZE.1,
                )));
            }
        }
        _ => {}
    }
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
        geometry: ViewGeometry::Normal(WINDOW_SIZE.0, WINDOW_SIZE.1),
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

    let geometry_pass_id = RenderPassTargetId::new();
    let aabb_pass_id = RenderPassTargetId::new();
    let ui_pass_id = RenderPassTargetId::new();
    let screen_pass_id = RenderPassTargetId::new();

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

        let gbuffer = Rc::new(GBuffer::new(UVec2::new(WINDOW_SIZE.0, WINDOW_SIZE.1)));
        let geometry_pass = GeometryPass::new(geometry_pass_id, gbuffer.clone());
        let aabb_pass = AABBPass::new(aabb_pass_id, gbuffer.clone());
        let ui_pass = UIPass::new(ui_pass_id, ui_stream);
        let screen_pass = ScreenPass::new(screen_pass_id, gbuffer.clone());
        Ok(RenderPipeline::new(construct_chain!(
            geometry_pass,
            screen_pass,
            aabb_pass,
            ui_pass,
        )))
    })
    .unwrap();
    renderer.attach_to_ecs(world);

    let e = world.spawn();
    world.insert(
        e,
        RenderPassIDs {
            geometry: geometry_pass_id,
            aabb: aabb_pass_id,
            ui: ui_pass_id,
            screen: screen_pass_id,
        },
    );

    let e = world.spawn();
    world.insert(
        e,
        CurrentGeometry {
            is_fullscreen: false,
        },
    );

    world.add_handler(fullscreen_handler);
    world.add_handler(map_shaders_handler);
    world.add_handler(viewport_resized_handler);
}
