// Do not display a console window on Windows
#![windows_subsystem = "windows"]

use crate::components::fcam::FreeCamera;
use crate::components::input::InputHolder;
use crate::logging::setup_logging;
use crate::systems::asset::setup_assets_system;
use crate::systems::asset_swap::{setup_asset_swap_system, AndThen, DropAllAssetsEvent};
use crate::systems::monitoring::setup_monitoring_system;
use crate::systems::objects::setup_objects_system;
use crate::systems::rendering::setup_rendering_system;
use dawn_ecs::main_loop::{synchronized_loop_with_monitoring, unsynchronized_loop_with_monitoring};
use dawn_graphics::input::{InputEvent, KeyCode};
use dawn_graphics::view::{ViewHandle, ViewSynchronization};
use dawn_util::rendezvous::Rendezvous;
use evenio::event::{Receiver, Sender};
use evenio::world::World;
use std::panic;
use log::info;

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

fn escape_handler(r: Receiver<InputEvent>, mut s: Sender<DropAllAssetsEvent>) {
    // info!("Input event: {:?}", r.event);
    match r.event {
        InputEvent::KeyRelease(KeyCode::Escape) => {
            s.send(DropAllAssetsEvent(AndThen::StopMainLoop));
        }
        InputEvent::KeyRelease(KeyCode::Function(5)) => {
            s.send(DropAllAssetsEvent(AndThen::ReloadAssets));
        }
        _ => {}
    }
}

fn main() {
    // For development, it's more convenient to see the panic messages in the console.
    #[cfg(not(debug_assertions))]
    panic::set_hook(Box::new(|info| {
        ViewHandle::error_box(
            "A fatal error occurred",
            &format!("The application has encountered a fatal error and needs to close.\n\nError details: {}", info),
        );
        eprintln!("Fatal error: {}", info);
        std::process::exit(1);
    }));
    #[cfg(debug_assertions)]
    setup_logging(log::LevelFilter::Debug, None, true);

    #[cfg(not(debug_assertions))]
    setup_logging(log::LevelFilter::Info, Some("app.log".into()), false);

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
            let before_frame = Rendezvous::new(2);
            let after_frame = Rendezvous::new(2);
            setup_rendering_system(
                &mut world,
                bindings,
                Some(ViewSynchronization {
                    before_frame: before_frame.clone(),
                    after_frame: after_frame.clone(),
                }),
            );
            synchronized_loop_with_monitoring(&mut world, before_frame, after_frame);
        }
    }
}
