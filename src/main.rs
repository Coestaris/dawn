use crate::components::fcam::FreeCamera;
use crate::components::input::InputHolder;
use crate::logging::CommonLogger;
use crate::systems::asset::setup_assets_system;
use crate::systems::asset_reload::setup_asset_reload_system;
use crate::systems::monitoring::setup_monitoring_system;
use crate::systems::objects::setup_objects_system;
use crate::systems::rendering::setup_rendering_system;
use dawn_ecs::{run_loop_with_monitoring, StopEventLoop};
use dawn_graphics::input::{InputEvent, KeyCode};
use evenio::event::{Receiver, Sender};
use evenio::world::World;
use log::info;

mod components;
mod logging;
mod systems;

// On my linux machine, the refresh rate is 60 Hz.
// I'll deal with it later
#[cfg(target_os = "linux")]
const REFRESH_RATE: f32 = 60.0;
#[cfg(not(target_os = "linux"))]
const REFRESH_RATE: f32 = 144.0;

fn escape_handler(r: Receiver<InputEvent>, mut s: Sender<StopEventLoop>) {
    // info!("Input event: {:?}", r.event);
    if let InputEvent::KeyRelease(KeyCode::Escape) = r.event {
        info!("Escape key pressed, stopping the event loop");
        s.send(StopEventLoop);
    }
}

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    // Setup the world and standalone components
    let mut world = World::new();
    InputHolder::new().attach_to_ecs(&mut world);
    FreeCamera::new().attach_to_ecs(&mut world);

    // Setup the systems
    let bindings = setup_assets_system(&mut world);
    setup_rendering_system(&mut world, bindings);
    setup_asset_reload_system(&mut world);
    setup_monitoring_system(&mut world);
    setup_objects_system(&mut world);
    world.add_handler(escape_handler);

    // Run the event loop
    run_loop_with_monitoring(&mut world, REFRESH_RATE);
}
