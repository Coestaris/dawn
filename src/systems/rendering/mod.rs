use crate::systems::rendering::aabb_pass::AABBPass;
use crate::systems::rendering::geometry_pass::GeometryPass;
use crate::REFRESH_RATE;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::TypedAsset;
use dawn_ecs::StopEventLoop;
use dawn_graphics::construct_chain;
use dawn_graphics::gl::entities::shader_program::ShaderProgram;
use dawn_graphics::input::InputEvent;
use dawn_graphics::passes::chain::ChainCons;
use dawn_graphics::passes::chain::ChainNil;
use dawn_graphics::passes::events::{RenderPassEvent, RenderPassTargetId};
use dawn_graphics::passes::pipeline::RenderPipeline;
use dawn_graphics::renderer::{Renderer, RendererBackendConfig};
use dawn_graphics::view::{PlatformSpecificViewConfig, ViewConfig};
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::prelude::World;
use glam::Mat4;
use crate::systems::asset::FactoryBindings;

mod aabb_pass;
mod geometry_pass;

#[derive(Debug, Clone)]
pub(crate) enum CustomPassEvent {
    DropAllAssets,
    UpdateShader(TypedAsset<ShaderProgram>),
    UpdateView(Mat4),
    UpdateWindowSize(usize, usize),
}

#[derive(Component)]
#[component(immutable)]
pub struct RenderPassIDs {
    pub geometry: RenderPassTargetId,
    pub aabb: RenderPassTargetId,
}

fn map_assets_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    ids: Single<&RenderPassIDs>,
    mut sender: Sender<(StopEventLoop, RenderPassEvent<CustomPassEvent>)>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) if *id == "geometry".into() => {
            let shader = hub.get_typed::<ShaderProgram>(id.clone()).unwrap();
            sender.send(RenderPassEvent::new(
                ids.geometry,
                CustomPassEvent::UpdateShader(shader),
            ));
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
                CustomPassEvent::UpdateWindowSize(*width, *height),
            ));
        }
        _ => {}
    }
}

pub fn setup_rendering_system(world: &mut World, bindings: FactoryBindings) {
    let win_size = (1920, 1080); // Default window size
    let view_config = ViewConfig {
        platform_specific: PlatformSpecificViewConfig {},
        title: "Hello world".to_string(),
        width: win_size.0,
        height: win_size.1,
    };

    let backend_config = RendererBackendConfig {
        fps: REFRESH_RATE as usize,
        shader_factory_binding: Some(bindings.shader),
        texture_factory_binding: Some(bindings.texture),
        mesh_factory_binding: Some(bindings.mesh),
        material_factory_binding: Some(bindings.material),
        vsync: true,
    };

    let geometry_pass_id = RenderPassTargetId::new();
    let aabb_pass_id = RenderPassTargetId::new();

    let renderer = Renderer::new_with_monitoring(view_config, backend_config, move |_| {
        let geometry_pass = GeometryPass::new(geometry_pass_id, win_size);
        let aabb_pass = AABBPass::new(aabb_pass_id);
        Ok(RenderPipeline::new(construct_chain!(
            geometry_pass,
            aabb_pass
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
        },
    );

    world.add_handler(map_assets_handler);
    world.add_handler(viewport_resized_handler);
}
