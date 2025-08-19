use crate::controller::GameController;
use crate::input::InputHolder;
use crate::rendering::CustomPassEvent;
use dawn_ecs::Tick;
use dawn_graphics::input::{KeyCode, MouseButton};
use dawn_graphics::passes::events::RenderPassEvent;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::world::World;
use glam::{Mat4, Quat, Vec3};

#[derive(Component)]
pub struct FreeCamera {
    position: Vec3,
    rotation: Quat,
}

impl FreeCamera {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }

    fn as_view(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.position)
    }

    pub fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);

        fn handler(
            r: Receiver<Tick>,
            holder: Single<&mut InputHolder>,
            mut cam: Single<&mut FreeCamera>,
            gc: Single<&mut GameController>,
            mut s: Sender<RenderPassEvent<CustomPassEvent>>,
        ) {
            const MOVE_SPEED: f32 = 30.0;
            let mut updated = false;
            let rotation = cam.rotation;
            if holder.key_pressed(KeyCode::Latin('W')) {
                cam.position += Vec3::Z * r.event.delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::Latin('S')) {
                cam.position += Vec3::Z * -r.event.delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::Latin('A')) {
                cam.position += Vec3::X * r.event.delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::Latin('D')) {
                cam.position += Vec3::X * -r.event.delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::Space) {
                cam.position += Vec3::Y * -r.event.delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::AltL) {
                cam.position += Vec3::Y * r.event.delta * MOVE_SPEED;
                updated = true;
            }
            if holder.button_pressed(MouseButton::Left) {
                let delta = holder.mouse_pos();
                cam.rotation = Quat::from_rotation_y(delta.x * 0.0001)
                    * Quat::from_rotation_x(delta.y * 0.0001)
                    * rotation;
                updated = true;
            }

            if updated {
                gc.on_view_update(cam.as_view(), &mut s);
            }
        }

        world.add_handler(handler.low());
    }
}
