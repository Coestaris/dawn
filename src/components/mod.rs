use crate::components::input::InputHolder;
use dawn_ecs::events::TickEvent;
use dawn_graphics::ecs::{ObjectPointLight, ObjectPosition, ObjectRotation};
use dawn_graphics::input::KeyCode;
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::{Fetcher, Single};
use glam::Quat;

pub mod dictionaries;
pub mod fcam;
pub mod imui;
pub mod input;

#[derive(Component)]
pub struct Rotating;

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
    f: Fetcher<(&mut ObjectPosition, &ObjectPointLight)>,
) {
    for (pos, _) in f {
        const SPEED: f32 = 10.0;
        if holder.key_pressed(KeyCode::Up) {
            pos.0.y += t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::Down) {
            pos.0.y -= t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::Left) {
            pos.0.x -= t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::Right) {
            pos.0.x += t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::PageUp) {
            pos.0.z += t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::PageDown) {
            pos.0.z -= t.event.delta * SPEED;
        }
    }
}
