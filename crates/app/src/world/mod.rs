use crate::world::input::InputHolder;
use dawn_ecs::events::TickEvent;
use dawn_graphics::ecs::{ObjectPosition, ObjectRotation};
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::{Fetcher, Single};
use glam::Quat;
use winit::keyboard::{KeyCode, PhysicalKey};

pub mod app_icon;
pub mod asset;
#[cfg(feature = "devtools")]
pub mod devtools;
pub mod exit;
pub mod fcam;
pub mod fullscreen;
pub mod input;
pub mod maps;

#[derive(Component)]
pub struct Rotating;
#[derive(Component)]
struct MovingByArrowKeys;

fn rotate_handler(t: Receiver<TickEvent>, f: Fetcher<(&mut ObjectRotation, &Rotating)>) {
    for (rot, _) in f {
        rot.0 = rot.0
            * Quat::from_rotation_y(t.event.delta * 0.3)
            * Quat::from_rotation_x(t.event.delta * 0.1);
    }
}

fn move_light_handler(
    t: Receiver<TickEvent>,
    holder: Single<&mut InputHolder>,
    f: Fetcher<(&mut ObjectPosition, &MovingByArrowKeys)>,
) {
    for (pos, _) in f {
        const SPEED: f32 = 10.0;
        if holder.key_pressed(PhysicalKey::Code(KeyCode::ArrowUp)) {
            pos.0.y += t.event.delta * SPEED;
        }
        if holder.key_pressed(PhysicalKey::Code(KeyCode::ArrowDown)) {
            pos.0.y -= t.event.delta * SPEED;
        }
        if holder.key_pressed(PhysicalKey::Code(KeyCode::ArrowLeft)) {
            pos.0.x -= t.event.delta * SPEED;
        }
        if holder.key_pressed(PhysicalKey::Code(KeyCode::ArrowRight)) {
            pos.0.x += t.event.delta * SPEED;
        }
        if holder.key_pressed(PhysicalKey::Code(KeyCode::PageUp)) {
            pos.0.z += t.event.delta * SPEED;
        }
        if holder.key_pressed(PhysicalKey::Code(KeyCode::PageDown)) {
            pos.0.z -= t.event.delta * SPEED;
        }
    }
}
