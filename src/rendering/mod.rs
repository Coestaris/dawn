use crate::assets::FactoryBindings;
use crate::rendering::aabb_pass::AABBPass;
use crate::rendering::geometry_pass::GeometryPass;
use crate::REFRESH_RATE;
use dawn_assets::factory::FactoryBinding;
use dawn_assets::TypedAsset;
use dawn_graphics::construct_chain;
use dawn_graphics::gl::entities::shader_program::ShaderProgram;
use dawn_graphics::passes::chain::ChainCons;
use dawn_graphics::passes::chain::ChainNil;
use dawn_graphics::passes::events::RenderPassTargetId;
use dawn_graphics::passes::pipeline::RenderPipeline;
use dawn_graphics::renderer::{Renderer, RendererBackendConfig};
use dawn_graphics::view::{PlatformSpecificViewConfig, ViewConfig};
use evenio::prelude::World;
use glam::Mat4;

mod aabb_pass;
mod geometry_pass;

#[derive(Debug, Clone)]
pub(crate) enum CustomPassEvent {
    UpdateShader(TypedAsset<ShaderProgram>),
    UpdateView(Mat4),
    UpdateWindowSize(usize, usize),
}

pub fn setup_graphics(
    world: &mut World,
    bindings: FactoryBindings,
) -> (RenderPassTargetId, RenderPassTargetId) {
    let view_config = ViewConfig {
        platform_specific: PlatformSpecificViewConfig {},
        title: "Hello world".to_string(),
        width: 800,
        height: 600,
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
        let geometry_pass = GeometryPass::new(geometry_pass_id, (800, 600));
        let aabb_pass = AABBPass::new(aabb_pass_id);
        Ok(RenderPipeline::new(construct_chain!(
            geometry_pass,
            aabb_pass
        )))
    })
    .unwrap();
    renderer.attach_to_ecs(world);

    (geometry_pass_id, aabb_pass_id)
}
