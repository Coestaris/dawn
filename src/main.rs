mod chain;
mod logging;

use crate::chain::{AABBPass, CustomPassEvent, GeometryPass};
use crate::logging::{format_system_time, CommonLogger};
use dawn_assets::factory::FactoryBinding;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::ir::IRAsset;
use dawn_assets::reader::AssetReader;
use dawn_assets::{AssetHeader, AssetID, AssetType};
use dawn_ecs::{run_loop_with_monitoring, MainLoopMonitoring, StopEventLoop, Tick};
use dawn_graphics::construct_chain;
use dawn_graphics::gl::entities::mesh::Mesh;
use dawn_graphics::gl::entities::shader_program::ShaderProgram;
use dawn_graphics::gl::entities::texture::Texture;
use dawn_graphics::input::{InputEvent, KeyCode};
use dawn_graphics::passes::chain::ChainCons;
use dawn_graphics::passes::chain::ChainNil;
use dawn_graphics::passes::events::{RenderPassEvent, RenderPassTargetId};
use dawn_graphics::passes::pipeline::RenderPipeline;
use dawn_graphics::renderable::{Position, RenderableMesh, Rotation, Scale};
use dawn_graphics::renderer::{Renderer, RendererBackendConfig, RendererMonitoring};
use dawn_graphics::view::{PlatformSpecificViewConfig, ViewConfig};
use dawn_yarc::Manifest;
use evenio::component::Component;
use evenio::event::{Insert, Receiver, Sender, Spawn};
use evenio::fetch::{Fetcher, Single};
use evenio::world::World;
use glam::*;
use log::{debug, error, info};
use std::collections::HashMap;
use std::path::PathBuf;

// On my linux machine, the refresh rate is 60 Hz.
// I'll deal with it later
#[cfg(target_os = "linux")]
const REFRESH_RATE: f32 = 60.0;
#[cfg(not(target_os = "linux"))]
const REFRESH_RATE: f32 = 144.0;

#[derive(Component)]
struct GameController {
    geometry_pass_id: RenderPassTargetId,
    aabb_pass_id: RenderPassTargetId,
}

impl GameController {
    fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
    }

    pub fn setup_asset_hub(world: &mut World) -> (FactoryBinding, FactoryBinding, FactoryBinding) {
        struct Reader;
        impl AssetReader for Reader {
            fn read(&mut self) -> Result<HashMap<AssetID, (AssetHeader, IRAsset)>, String> {
                let yarc = env!("YARC_FILE");
                info!("Reading assets from: {}", yarc);

                let (manifest, assets) = dawn_yarc::read(PathBuf::from(yarc)).unwrap();
                #[rustfmt::skip]
                fn log(manifest: Manifest) {
                    debug!("> Version: {}", manifest.version.unwrap_or("unknown".to_string()));
                    debug!("> Author: {}", manifest.author.unwrap_or("unknown".to_string()));
                    debug!("> Description: {}", manifest.description.unwrap_or("No description".to_string()));
                    debug!("> License: {}", manifest.license.unwrap_or("No license".to_string()));
                    debug!("> Created: {}", format_system_time(manifest.created).unwrap());
                    debug!("> Tool: {} (version {})", manifest.tool, manifest.tool_version);
                    debug!("> Serializer: {} (version {})", manifest.serializer, manifest.serializer_version);
                    debug!("> Assets: {}", manifest.headers.len());
                }
                // Move manifest to the logger.
                // There's no better use for it.
                log(manifest);

                let mut result = HashMap::new();
                for (header, ir) in assets {
                    result.insert(header.id.clone(), (header, ir));
                }

                Ok(result)
            }
        }
        let mut hub = AssetHub::new(Reader).unwrap();

        // Unlike other factories, shader and texture assets are
        // managed directly by the renderer, instead of processing assets
        // in the main loop (via ECS).
        let shader_binding = hub.create_factory_biding(AssetType::Shader);
        let texture_binding = hub.create_factory_biding(AssetType::Texture);
        let mesh_binding = hub.create_factory_biding(AssetType::Mesh);

        hub.query_load_all().unwrap();
        hub.attach_to_ecs(world);

        (shader_binding, texture_binding, mesh_binding)
    }

    pub fn setup_graphics(
        world: &mut World,
        shader_binding: FactoryBinding,
        texture_binding: FactoryBinding,
        mesh_binding: FactoryBinding,
    ) -> (RenderPassTargetId, RenderPassTargetId) {
        let view_config = ViewConfig {
            platform_specific: PlatformSpecificViewConfig {},
            title: "Hello world".to_string(),
            width: 800,
            height: 600,
        };

        let backend_config = RendererBackendConfig {
            fps: REFRESH_RATE as usize,
            shader_factory_binding: Some(shader_binding),
            texture_factory_binding: Some(texture_binding),
            mesh_factory_binding: Some(mesh_binding),
            vsync: true,
        };

        let geometry_pass_id = RenderPassTargetId::new();
        let aabb_pass_id = RenderPassTargetId::new();

        let renderer = Renderer::new_with_monitoring(view_config, backend_config, move |_| {
            let geometry_pass = GeometryPass::new(geometry_pass_id, (800, 600));
            let aabb_pass = AABBPass::new(aabb_pass_id);
            Ok(RenderPipeline::new(construct_chain!(
                geometry_pass,
                aabb_pass
            )))
        })
        .unwrap();
        renderer.attach_to_ecs(world);

        (geometry_pass_id, aabb_pass_id)
    }

    pub fn setup(world: &mut World) {
        let (shader, texture, mesh) = Self::setup_asset_hub(world);
        let (geometry_pass, aabb_pass) = Self::setup_graphics(world, shader, texture, mesh);
        GameController {
            geometry_pass_id: geometry_pass,
            aabb_pass_id: aabb_pass,
        }
        .attach_to_ecs(world);
    }
}

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
        AssetHubEvent::LoadFailed(_, _, _) => {
            error!("Aborting due to asset load failure");
            stopper.send(StopEventLoop);
        }
        AssetHubEvent::AllAssetsFreed => {
            info!("All assets have been freed");
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
        AssetHubEvent::AllAssetsLoaded => {
            rpe.send(RenderPassEvent::new(
                gc.geometry_pass_id,
                CustomPassEvent::UpdateShader(
                    hub.get_typed::<ShaderProgram>(AssetID::from("triangle"))
                        .unwrap(),
                ),
            ));
        }
        _ => {}
    }
}

fn assets_spawn(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    mut spawn: Sender<(
        Spawn,
        Insert<Position>,
        Insert<Scale>,
        Insert<RenderableMesh>,
        Insert<Rotation>,
    )>,
) {
    match r.event {
        AssetHubEvent::AllAssetsLoaded => {
            info!("Spawning a teapot mesh");
            let id = spawn.spawn();
            spawn.insert(
                id,
                RenderableMesh(hub.get_typed::<Mesh>(AssetID::from("teapot")).unwrap()),
            );
            spawn.insert(id, Rotation(Quat::IDENTITY));
            spawn.insert(id, Scale(Vec3::splat(1.0)));
            spawn.insert(id, Position(Vec3::new(0.0, 0.0, -5.0)));
        }
        _ => {}
    }
}

fn rotate_handler(t: Receiver<Tick>, mut rotation: Fetcher<&mut Rotation>) {
    for f in rotation {
        f.0 = f.0 * Quat::from_rotation_y(t.event.delta);
    }
}

fn events_handler(
    ie: Receiver<InputEvent>,
    gc: Single<&mut GameController>,
    mut s: Sender<RenderPassEvent<CustomPassEvent>>,
) {
    match ie.event {
        InputEvent::Resize { width, height } => {
            info!("Window resized to {}x{}", width, height);
            s.send(RenderPassEvent::new(
                gc.geometry_pass_id,
                CustomPassEvent::UpdateWindowSize(*width, *height),
            ));
        }
        _ => {}
    }
}

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let mut world = World::new();
    GameController::setup(&mut world);

    // Core handlers
    world.add_handler(main_loop_profile_handler);
    world.add_handler(renderer_profile_handler);
    world.add_handler(escape_handler);

    // Asset handlers
    world.add_handler(assets_failed_handler);
    world.add_handler(assets_loaded_handler);

    world.add_handler(events_handler);
    world.add_handler(assets_spawn);
    world.add_handler(rotate_handler);

    run_loop_with_monitoring(&mut world, REFRESH_RATE);
}
