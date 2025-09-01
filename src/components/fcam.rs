use crate::components::input::InputHolder;
use crate::systems::rendering::{CustomPassEvent, RenderPassIDs};
use dawn_ecs::events::TickEvent;
use dawn_graphics::input::{InputEvent, KeyCode, MouseButton};
use dawn_graphics::passes::events::RenderPassEvent;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::world::World;
use glam::{FloatExt, Mat4, Vec2, Vec3};
use std::f32::consts::PI;

pub struct CameraData {
    position: Vec3,
    pitch: f32,
    yaw: f32,
}

impl CameraData {
    /// The smallest angle that can be considered "zero"
    const EPS: f32 = 0.0001;

    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            pitch: 0.0,
            yaw: PI / 2.0,
        }
    }

    pub fn as_view(&self) -> Mat4 {
        Mat4::look_to_lh(self.position, self.as_direction(), Vec3::Y)
    }

    pub fn as_direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, t),
            pitch: self.pitch.lerp(other.pitch, t),
            yaw: self.yaw.lerp(other.yaw, t),
        }
    }
}

impl PartialEq<Self> for CameraData {
    fn eq(&self, other: &Self) -> bool {
        (self.position - other.position).length() < Self::EPS
            && (self.pitch - other.pitch).abs() < Self::EPS
            && (self.yaw - other.yaw).abs() < Self::EPS
    }
}

#[derive(Component)]
pub struct FreeCamera {
    click_pos: Vec2,
    data: CameraData,
    instant: CameraData,
}

impl FreeCamera {
    pub fn new() -> Self {
        Self {
            click_pos: Vec2::ZERO,
            data: CameraData::new(),
            instant: CameraData::new(),
        }
    }

    fn as_view(&self) -> Mat4 {
        self.data.as_view()
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
            r: Receiver<TickEvent>,
            holder: Single<&mut InputHolder>,
            mut cam: Single<&mut FreeCamera>,
            ids: Single<&RenderPassIDs>,
            mut sender: Sender<RenderPassEvent<CustomPassEvent>>,
        ) {
            const MOVE_SPEED: f32 = 25.0;
            const ROTATE_SPEED: f32 = 0.001;
            const LERP: f32 = 0.5;

            let delta = r.event.delta;

            let direction = cam.instant.as_direction();
            let right = direction.cross(Vec3::Y).normalize();
            let up = direction.cross(right).normalize();

            if holder.key_pressed(KeyCode::Latin('W')) {
                cam.instant.position += direction * -delta * MOVE_SPEED;
            }
            if holder.key_pressed(KeyCode::Latin('S')) {
                cam.instant.position += direction * delta * MOVE_SPEED;
            }
            if holder.key_pressed(KeyCode::Latin('A')) {
                cam.instant.position += right * delta * MOVE_SPEED;
            }
            if holder.key_pressed(KeyCode::Latin('D')) {
                cam.instant.position += right * -delta * MOVE_SPEED;
            }
            if holder.key_pressed(KeyCode::Space) {
                cam.instant.position += up * -delta * MOVE_SPEED;
            }
            if holder.key_pressed(KeyCode::ShiftL) {
                cam.instant.position += up * delta * MOVE_SPEED;
            }
            if holder.button_pressed(MouseButton::Left) {
                let pos_delta = holder.mouse_pos() - cam.click_pos;
                cam.click_pos = holder.mouse_pos();

                // Allow look around in all directions
                cam.instant.yaw = cam.instant.yaw - pos_delta.x * ROTATE_SPEED;
                // Clamp pitch to prevent gimbal lock
                cam.instant.pitch = (cam.instant.pitch - pos_delta.y * ROTATE_SPEED)
                    .clamp(-PI / 2.0 + 0.01, PI / 2.0 - 0.01);
            }

            // Smoothly interpolate position and rotation
            let data = cam.data.lerp(&cam.instant, LERP * delta * 30.0);
            if cam.data != data {
                cam.data = data;

                let broadcast = [ids.geometry, ids.aabb];
                for id in broadcast.iter() {
                    sender.send(RenderPassEvent::new(
                        *id,
                        CustomPassEvent::UpdateView(cam.as_view()),
                    ));
                }
            }
        }

        world.add_handler(tick_handler.low());
        world.add_handler(input_handler.low());
    }
}
