use crate::systems::asset::FactoryBindings;
use crate::systems::rendering::aabb_pass::AABBPass;
use crate::systems::rendering::geometry_pass::GeometryPass;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::TypedAsset;
use dawn_ecs::events::ExitEvent;
use dawn_graphics::construct_chain;
use dawn_graphics::gl::bindings;
use dawn_graphics::gl::font::Font;
use dawn_graphics::gl::raii::shader_program::ShaderProgram;
use dawn_graphics::input::InputEvent;
use dawn_graphics::passes::chain::ChainCons;
use dawn_graphics::passes::chain::ChainNil;
use dawn_graphics::passes::events::{RenderPassEvent, RenderPassTargetId};
use dawn_graphics::passes::pipeline::RenderPipeline;
use dawn_graphics::renderer::{Renderer, RendererBackendConfig};
use dawn_graphics::view::{PlatformSpecificViewConfig, ViewConfig, ViewSynchronization};
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::prelude::World;
use glam::{Mat4, UVec2};
use std::collections::HashMap;

mod aabb_pass;
mod geometry_pass;
mod ui_pass;

#[derive(Debug, Clone)]
pub(crate) enum CustomPassEvent {
    DropAllAssets,
    UpdateShader(TypedAsset<ShaderProgram>),
    UpdateFont(TypedAsset<Font>),
    UpdateView(Mat4),
    UpdateWindowSize(UVec2),
}

#[derive(Component)]
#[component(immutable)]
pub struct RenderPassIDs {
    pub geometry: RenderPassTargetId,
    pub aabb: RenderPassTargetId,
    pub ui: RenderPassTargetId,
}

fn map_shaders_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    ids: Single<&RenderPassIDs>,
    mut sender: Sender<(ExitEvent, RenderPassEvent<CustomPassEvent>)>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) => {
            let map = HashMap::from([("geometry_shader", ids.geometry), ("glyph_shader", ids.ui)]);

            if let Some(target) = map.get(id.as_str()) {
                let shader = hub.get_typed::<ShaderProgram>(id.clone()).unwrap();
                sender.send(RenderPassEvent::new(
                    *target,
                    CustomPassEvent::UpdateShader(shader),
                ));
            } else if *id == "arial_geo".into() {
                let font = hub.get_typed::<Font>(id.clone()).unwrap();
                sender.send(RenderPassEvent::new(
                    ids.ui,
                    CustomPassEvent::UpdateFont(font),
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
            sender.send(RenderPassEvent::new(
                ids.geometry,
                CustomPassEvent::UpdateWindowSize(UVec2::new(*width as u32, *height as u32)),
            ));
            sender.send(RenderPassEvent::new(
                ids.ui,
                CustomPassEvent::UpdateWindowSize(UVec2::new(*width as u32, *height as u32)),
            ));
        }
        _ => {}
    }
}

pub fn setup_rendering_system(
    world: &mut World,
    bindings: FactoryBindings,
    synchronization: Option<ViewSynchronization>,
) {
    let win_size = (1280, 720); // Default window size
    let view_config = ViewConfig {
        platform_specific: PlatformSpecificViewConfig {},
        title: "Hello world".to_string(),
        width: win_size.0,
        height: win_size.1,
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

    let renderer = Renderer::new_with_monitoring(view_config, backend_config, move |_| {
        // Setup OpenGL state
        unsafe {
            // Enable wireframe mode
            // bindings::PolygonMode(bindings::FRONT_AND_BACK, bindings::LINE);

            bindings::ShadeModel(bindings::SMOOTH);
            bindings::Enable(bindings::DEPTH_TEST);
            bindings::DepthFunc(bindings::LEQUAL);
            bindings::Enable(bindings::MULTISAMPLE);
            bindings::Hint(bindings::PERSPECTIVE_CORRECTION_HINT, bindings::NICEST);
            bindings::Enable(bindings::BLEND);
            bindings::BlendFunc(bindings::SRC_ALPHA, bindings::ONE_MINUS_SRC_ALPHA);
        }

        let geometry_pass = GeometryPass::new(geometry_pass_id);
        let aabb_pass = AABBPass::new(aabb_pass_id);
        let ui_pass = ui_pass::UIPass::new(ui_pass_id);
        Ok(RenderPipeline::new(construct_chain!(
            geometry_pass,
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
        },
    );

    world.add_handler(map_shaders_handler);
    world.add_handler(viewport_resized_handler);
}
