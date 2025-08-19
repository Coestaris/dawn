use crate::rendering::CustomPassEvent;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::entities::shader_program::ShaderProgram;
use dawn_graphics::passes::events::{RenderPassEvent, RenderPassTargetId};
use evenio::component::Component;
use evenio::event::Sender;
use evenio::prelude::World;
use glam::Mat4;
use tinyrand::{Rand, Wyrand};

#[derive(Component)]
pub struct GameController {
    geometry_pass_id: RenderPassTargetId,
    aabb_pass_id: RenderPassTargetId,
    rand: Wyrand,
}

impl GameController {
    pub fn new(geometry_pass_id: RenderPassTargetId, aabb_pass_id: RenderPassTargetId) -> Self {
        GameController {
            geometry_pass_id,
            aabb_pass_id,
            rand: Wyrand::default(),
        }
    }

    pub fn rand_float(&mut self) -> f32 {
        // Generate a random float in the range [0.0, 1.0)
        self.rand.next_u32() as f32 / (std::u32::MAX as f32)
    }

    pub fn on_new_geometry_shader(
        &self,
        shader: TypedAsset<ShaderProgram>,
        sender: &mut Sender<RenderPassEvent<CustomPassEvent>>,
    ) {
        sender.send(RenderPassEvent::new(
            self.geometry_pass_id,
            CustomPassEvent::UpdateShader(shader),
        ));
    }

    pub fn on_view_update(
        &self,
        view: Mat4,
        sender: &mut Sender<RenderPassEvent<CustomPassEvent>>,
    ) {
        sender.send(RenderPassEvent::new(
            self.geometry_pass_id,
            CustomPassEvent::UpdateView(view),
        ));
    }

    pub fn on_resize(
        &self,
        sender: &mut Sender<RenderPassEvent<CustomPassEvent>>,
        width: usize,
        height: usize,
    ) {
        sender.send(RenderPassEvent::new(
            self.geometry_pass_id,
            CustomPassEvent::UpdateWindowSize(width, height),
        ));
    }

    pub fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
    }
}
