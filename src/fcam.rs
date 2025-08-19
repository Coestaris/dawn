use crate::controller::GameController;
use crate::input::InputHolder;
use crate::rendering::CustomPassEvent;
use dawn_ecs::Tick;
use dawn_graphics::input::{InputEvent, KeyCode, MouseButton};
use dawn_graphics::passes::events::RenderPassEvent;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::world::World;
use glam::{Mat4, Quat, Vec2, Vec3};
use log::info;
use std::f32::consts::PI;

#[derive(Component)]
pub struct FreeCamera {
    position: Vec3,
    click_pos: Vec2,
    pitch: f32,
    yaw: f32,
}

impl FreeCamera {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            click_pos: Default::default(),
            pitch: 0.0,
            yaw: PI / 2.0,
        }
    }

    fn as_direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    fn as_view(&self) -> Mat4 {
        Mat4::look_to_lh(self.position, self.as_direction(), Vec3::Y)
    }

    pub fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);

        fn input_handler(
            r: Receiver<InputEvent>,
            holder: Single<&mut InputHolder>,
            mut cam: Single<&mut FreeCamera>,
        ) {
            match r.event {
                InputEvent::MouseButtonPress(MouseButton::Left) => {
                    cam.click_pos = holder.mouse_pos();
                }
                _ => {}
            }
        }

        fn tick_handler(
            r: Receiver<Tick>,
            holder: Single<&mut InputHolder>,
            mut cam: Single<&mut FreeCamera>,
            gc: Single<&mut GameController>,
            mut s: Sender<RenderPassEvent<CustomPassEvent>>,
        ) {
            const MOVE_SPEED: f32 = 45.0;
            const ROTATE_SPEED: f32 = 0.002;
            let delta = r.event.delta;
            let direction = cam.as_direction();
            let right = direction.cross(Vec3::Y).normalize();
            let up = direction.cross(right).normalize();

            let mut updated = false;
            if holder.key_pressed(KeyCode::Latin('W')) {
                cam.position += direction * -delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::Latin('S')) {
                cam.position += direction * delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::Latin('A')) {
                cam.position += right * delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::Latin('D')) {
                cam.position += right * -delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::Space) {
                cam.position += up * -delta * MOVE_SPEED;
                updated = true;
            }
            if holder.key_pressed(KeyCode::ShiftL) {
                cam.position += up * delta * MOVE_SPEED;
                updated = true;
            }
            if holder.button_pressed(MouseButton::Left) {
                let pos_delta = holder.mouse_pos() - cam.click_pos;
                cam.click_pos = holder.mouse_pos();
                // Allow look around in all directions
                cam.yaw = cam.yaw - pos_delta.x * ROTATE_SPEED;
                // Clamp pitch to prevent gimbal lock
                cam.pitch = (cam.pitch - pos_delta.y * ROTATE_SPEED)
                    .clamp(-PI / 2.0 + 0.01, PI / 2.0 - 0.01);
                updated = true;
            }

            if updated {
                gc.on_view_update(cam.as_view(), &mut s);
            }
        }

        world.add_handler(tick_handler.low());
        world.add_handler(input_handler.low());
    }
}
