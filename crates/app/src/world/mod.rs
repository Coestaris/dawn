use crate::assets::reader::ReaderBackend;
use crate::devtools::DevtoolsWorldConnection;
use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::RenderingEvent;
use crate::world::app_icon::map_app_icon_handler;
use crate::world::asset::setup_assets_system;
use crate::world::exit::escape_handler;
use crate::world::fcam::FreeCamera;
use crate::world::fullscreen::setup_fullscreen_system;
use crate::world::input::InputHolder;
use crate::world::maps::setup_maps_system;
use crate::world::skybox::map_skybox;
use dawn_assets::hub::AssetHub;
use dawn_ecs::events::TickEvent;
use dawn_graphics::ecs::{ObjectPosition, ObjectRotation};
use dawn_graphics::renderer::RendererProxy;
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::{Fetcher, Single};
use evenio::prelude::World;
use glam::Quat;
use std::sync::Arc;
use winit::keyboard::{KeyCode, PhysicalKey};

mod app_icon;
mod asset;
#[cfg(feature = "devtools")]
pub mod devtools;
mod exit;
mod fcam;
mod fullscreen;
mod input;
mod maps;
mod skybox;

#[derive(Component)]
pub struct Rotating;
#[derive(Component)]
pub struct MovingByArrowKeys;

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

pub struct MainToEcs {
    pub reader_backend: Arc<dyn ReaderBackend>,
    pub hub: AssetHub,
    pub renderer_proxy: RendererProxy<RenderingEvent>,
    pub dispatcher: RenderDispatcher,
    #[cfg(feature = "devtools")]
    pub devtools_connection: DevtoolsWorldConnection,
}

pub fn init_world(world: &mut World, to_ecs: MainToEcs) {
    to_ecs.renderer_proxy.attach_to_ecs(world);
    to_ecs.dispatcher.attach_to_ecs(world);

    InputHolder::new().attach_to_ecs(world);
    FreeCamera::new().attach_to_ecs(world);

    setup_assets_system(world, to_ecs.reader_backend, to_ecs.hub);
    setup_maps_system(world);
    setup_fullscreen_system(world);

    #[cfg(feature = "devtools")]
    {
        use crate::world::devtools::setup_devtools_system;
        setup_devtools_system(world, to_ecs.devtools_connection);
    }

    world.add_handler(escape_handler);
    world.add_handler(map_app_icon_handler);
    world.add_handler(map_skybox);
}
