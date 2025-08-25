use crate::components::fcam::FreeCamera;
use crate::components::input::InputHolder;
use crate::logging::CommonLogger;
use crate::systems::asset::setup_assets_system;
use crate::systems::asset_swap::{setup_asset_swap_system, AndThen, DropAllAssets};
use crate::systems::monitoring::setup_monitoring_system;
use crate::systems::objects::setup_objects_system;
use crate::systems::rendering::setup_rendering_system;
use dawn_ecs::{synchronized_loop_with_monitoring, unsynchronized_loop_with_monitoring};
use dawn_graphics::input::{InputEvent, KeyCode};
use evenio::event::{Receiver, Sender};
use evenio::world::World;
use dawn_util::rendezvous::Rendezvous;

mod components;
mod logging;
mod systems;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum WorldSyncMode {
    SynchronizedWithMonitor,
    FixedTickRate(usize),
}

const WORLD_SYNC_MODE: WorldSyncMode = WorldSyncMode::SynchronizedWithMonitor;

// #[cfg(target_os = "linux")]
// const WORLD_SYNC_MODE: WorldSyncMode = WorldSyncMode::FixedTickRate(60);
// #[cfg(not(target_os = "linux"))]
// const WORLD_SYNC_MODE: WorldSyncMode = WorldSyncMode::SynchronizedWithMonitor;

fn escape_handler(r: Receiver<InputEvent>, mut s: Sender<DropAllAssets>) {
    // info!("Input event: {:?}", r.event);
    match r.event {
        InputEvent::KeyRelease(KeyCode::Escape) => {
            s.send(DropAllAssets(AndThen::StopMainLoop));
        }
        InputEvent::KeyRelease(KeyCode::Function(5)) => {
            s.send(DropAllAssets(AndThen::ReloadAssets));
        }
        _ => {}
    }
}

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    // Setup the world and standalone components
    let mut world = World::new();
    InputHolder::new().attach_to_ecs(&mut world);
    FreeCamera::new().attach_to_ecs(&mut world);

    // Setup the systems
    setup_asset_swap_system(&mut world);
    setup_monitoring_system(&mut world);
    setup_objects_system(&mut world);
    world.add_handler(escape_handler);
    let bindings = setup_assets_system(&mut world);

    // Run the event loop
    match WORLD_SYNC_MODE {
        WorldSyncMode::FixedTickRate(tps) => {
            setup_rendering_system(&mut world, bindings, None);
            unsynchronized_loop_with_monitoring(&mut world, tps as f32);
        }
        WorldSyncMode::SynchronizedWithMonitor => {
            let rendezvous = Rendezvous::new(2);
            setup_rendering_system(&mut world, bindings, Some(rendezvous.clone()));
            synchronized_loop_with_monitoring(&mut world, rendezvous);
        }
    }
}
