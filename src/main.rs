use crate::assets::setup_asset_hub;
use crate::controller::GameController;
use crate::fcam::FreeCamera;
use crate::input::InputHolder;
use crate::logging::CommonLogger;
use crate::rendering::{setup_graphics, CustomPassEvent};
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_ecs::{run_loop_with_monitoring, MainLoopMonitoring, StopEventLoop, Tick};
use dawn_graphics::gl::entities::material::Material;
use dawn_graphics::gl::entities::mesh::Mesh;
use dawn_graphics::gl::entities::shader_program::ShaderProgram;
use dawn_graphics::input::{InputEvent, KeyCode};
use dawn_graphics::passes::events::RenderPassEvent;
use dawn_graphics::renderable::{
    ObjectMaterial, ObjectMesh, ObjectPosition, ObjectRotation, ObjectScale,
};
use dawn_graphics::renderer::RendererMonitoring;
use evenio::event::{Insert, Receiver, Sender, Spawn};
use evenio::fetch::{Fetcher, Single};
use evenio::world::World;
use glam::{Quat, Vec3};
use log::{error, info};

mod assets;
mod controller;
mod fcam;
mod input;
mod logging;
mod rendering;

// On my linux machine, the refresh rate is 60 Hz.
// I'll deal with it later
#[cfg(target_os = "linux")]
const REFRESH_RATE: f32 = 60.0;
#[cfg(not(target_os = "linux"))]
const REFRESH_RATE: f32 = 144.0;

fn main_loop_profile_handler(r: Receiver<MainLoopMonitoring>) {
    info!(
        "Main loop: {:.1}tps ({:.1}%)",
        r.event.tps.average(),
        r.event.load.average() * 100.0
    );
}

fn renderer_profile_handler(r: Receiver<RendererMonitoring>) {
    info!(
        "Renderer: {:.1} FPS. {:.1}/{:.1}",
        r.event.fps.average(),
        r.event.render.average().as_millis(),
        r.event.view.average().as_millis(),
    );
}

fn escape_handler(r: Receiver<InputEvent>, mut s: Sender<StopEventLoop>) {
    // info!("Input event: {:?}", r.event);
    if let InputEvent::KeyRelease(KeyCode::Escape) = r.event {
        info!("Escape key pressed, stopping the event loop");
        s.send(StopEventLoop);
    }
}

fn assets_failed_handler(r: Receiver<AssetHubEvent>, mut stopper: Sender<StopEventLoop>) {
    match r.event {
        AssetHubEvent::QueryCompleted(_, false) => {
            error!("Asset query failed, stopping the event loop");
            stopper.send(StopEventLoop);
        }
        AssetHubEvent::AssetFailed(aid, error) => {
            let error = match error {
                Some(e) => e.to_string(),
                None => "Unknown error".to_string(),
            };
            error!("Aborting due to asset load failure: {}: {}", aid, error);
            stopper.send(StopEventLoop);
        }
        _ => {}
    }
}

fn assets_loaded_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    gc: Single<&GameController>,
    mut rpe: Sender<RenderPassEvent<CustomPassEvent>>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) if *id == "geometry".into() => {
            let shader = hub.get_typed::<ShaderProgram>(id.clone()).unwrap();
            gc.on_new_geometry_shader(shader.clone(), &mut rpe);
        }
        _ => {}
    }
}

fn assets_spawn(
    r: Receiver<AssetHubEvent>,
    mut gc: Single<&mut GameController>,
    hub: Single<&mut AssetHub>,
    mut spawn: Sender<(
        Spawn,
        Insert<ObjectPosition>,
        Insert<ObjectScale>,
        Insert<ObjectMesh>,
        Insert<ObjectRotation>,
        Insert<ObjectMaterial>,
    )>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) if *id == "barrel".into() => {
            for i in 0..1 {
                let entity = spawn.spawn();
                spawn.insert(
                    entity,
                    ObjectMesh(hub.get_typed::<Mesh>(id.clone()).unwrap()),
                );
                spawn.insert(entity, ObjectRotation(Quat::IDENTITY));
                spawn.insert(
                    entity,
                    ObjectScale(Vec3::splat(gc.rand_float() * 0.5 + 0.8)),
                );
                spawn.insert(entity, ObjectPosition(Vec3::new(0.0, 0.0, -10.0)));
                spawn.insert(
                    entity,
                    ObjectMaterial(hub.get_typed::<Material>("barrel_material".into()).unwrap()),
                );
            }
        }
        _ => {}
    }
}

fn rotate_handler(t: Receiver<Tick>, rotation: Fetcher<&mut ObjectRotation>) {
    // for f in rotation {
    //     f.0 =
    //         f.0 * Quat::from_rotation_y(t.event.delta) * Quat::from_rotation_x(t.event.delta * 0.5);
    // }
}

fn events_handler(
    ie: Receiver<InputEvent>,
    gc: Single<&mut GameController>,
    mut s: Sender<RenderPassEvent<CustomPassEvent>>,
) {
    match ie.event {
        InputEvent::Resize { width, height } => {
            gc.on_resize(&mut s, *width, *height);
        }
        _ => {}
    }
}

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let mut world = World::new();

    let bindings = setup_asset_hub(&mut world);
    let (geometry_pass, aabb_pass) = setup_graphics(&mut world, bindings);
    GameController::new(geometry_pass, aabb_pass).attach_to_ecs(&mut world);
    InputHolder::new().attach_to_ecs(&mut world);
    FreeCamera::new().attach_to_ecs(&mut world);

    world.add_handler(main_loop_profile_handler);
    world.add_handler(renderer_profile_handler);
    world.add_handler(escape_handler);
    world.add_handler(assets_failed_handler);
    world.add_handler(assets_loaded_handler);
    world.add_handler(events_handler);
    world.add_handler(assets_spawn);
    world.add_handler(rotate_handler);

    run_loop_with_monitoring(&mut world, REFRESH_RATE);
}
