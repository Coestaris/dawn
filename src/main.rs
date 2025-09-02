// Do not display a console window on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::components::fcam::FreeCamera;
use crate::components::input::InputHolder;
use crate::logging::setup_logging;
use crate::systems::asset::setup_assets_system;
use crate::systems::asset_swap::{setup_asset_swap_system, AndThen, DropAllAssetsEvent};
use crate::systems::objects::setup_objects_system;
use crate::systems::ui::setup_ui_system;
use dawn_ecs::main_loop::{synchronized_loop_with_monitoring, unsynchronized_loop_with_monitoring};
use dawn_graphics::input::{InputEvent, KeyCode};
use dawn_graphics::view::ViewSynchronization;
use dawn_util::rendezvous::Rendezvous;
use evenio::event::{Receiver, Sender};
use evenio::world::World;
use log::{error, info};
use std::panic;
use crate::rendering::setup_rendering_system;

mod components;
mod logging;
mod systems;
pub mod rendering;

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

fn panic_hook(info: &panic::PanicHookInfo) {
    // For development, it's more convenient to see the panic messages in the console.
    #[cfg(not(debug_assertions))]
    {
        use dawn_graphics::view::ViewHandle;
        ViewHandle::error_box(
            "A fatal error occurred",
            &format!("The application has encountered a fatal error and needs to close.\n\nError details: {}", info),
        );
    }

    error!("Panic: {}", info);
}

fn main() {
    // Disable colors in the release builds to not consume extra resources.
    // It also makes the log files much more readable.
    #[cfg(not(debug_assertions))]
    setup_logging(log::LevelFilter::Info, Some("dawn_log".into()), false);

    #[cfg(debug_assertions)]
    setup_logging(log::LevelFilter::Info, None, true);

    // Set up the world and standalone components
    let mut world = World::new();
    InputHolder::new().attach_to_ecs(&mut world);
    FreeCamera::new().attach_to_ecs(&mut world);

    // Setup the systems
    setup_asset_swap_system(&mut world);
    setup_objects_system(&mut world);
    world.add_handler(escape_handler);
    let bindings = setup_assets_system(&mut world);
    let ui_stream = setup_ui_system(&mut world);

    // Run the event loop
    match WORLD_SYNC_MODE {
        WorldSyncMode::FixedTickRate(tps) => {
            panic::set_hook(Box::new(|info| {
                panic_hook(info);
            }));

            setup_rendering_system(&mut world, bindings, None, ui_stream);
            unsynchronized_loop_with_monitoring(&mut world, tps as f32);
        }
        WorldSyncMode::SynchronizedWithMonitor => {
            let before_frame = Rendezvous::new(2);
            let after_frame = Rendezvous::new(2);

            {
                // We need to leak the rendezvous points to make sure they
                // live for the entire duration of the program.
                let before_frame = Box::leak(Box::new(before_frame.clone()));
                let after_frame = Box::leak(Box::new(after_frame.clone()));

                panic::set_hook(Box::new(|info| {
                    panic_hook(info);

                    // TODO: Maybe move this to the library side?
                    // In case of a panic, we want to make sure that both threads can exit cleanly.
                    // So we signal both rendezvous points to avoid deadlocks.
                    before_frame.unlock();
                    after_frame.unlock();
                }));
            }

            setup_rendering_system(
                &mut world,
                bindings,
                Some(ViewSynchronization {
                    before_frame: before_frame.clone(),
                    after_frame: after_frame.clone(),
                }),
                ui_stream
            );
            synchronized_loop_with_monitoring(&mut world, before_frame, after_frame);
        }
    }

    info!("Main loop has exited");
}
