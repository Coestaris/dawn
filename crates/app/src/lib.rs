#[cfg(feature = "devtools")]
pub mod devtools;
pub mod logging;
pub mod rendering;
pub mod world;

use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::RenderingEvent;
use crate::rendering::{setup_rendering, SetupRenderingParameters};
use crate::world::app_icon::map_app_icon_handler;
use crate::world::asset::setup_assets_system;
use crate::world::exit::escape_handler;
use crate::world::fcam::FreeCamera;
use crate::world::fullscreen::setup_fullscreen_system;
use crate::world::input::InputHolder;
use crate::world::maps::setup_maps_system;
use build_info::BuildInfo;
use dawn_assets::hub::AssetHub;
use dawn_assets::AssetType;
use dawn_ecs::world::WorldLoopProxy;
use dawn_graphics::renderer::{
    Renderer, RendererConfig, RendererProxy, RendererSynchronization, WindowConfig,
};
use dawn_util::rendezvous::Rendezvous;
use evenio::prelude::World;
use glam::UVec2;
use log::{error, info};
use std::backtrace::BacktraceStatus;
use std::panic;
use std::panic::PanicHookInfo;
use web_time::Instant;
use winit::window::{Cursor, CursorIcon};

#[cfg(feature = "devtools")]
use crate::devtools::{devtools_bridge, DevtoolsWorldConnection};
use crate::logging::{print_build_info, START_TIME};

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum WorldSyncMode {
    SynchronizedWithMonitor,
    FixedTickRate(usize),
}

pub(crate) static WINDOW_SIZE: UVec2 = UVec2::new(1280, 720);

struct MainToEcs {
    hub: AssetHub,
    renderer_proxy: RendererProxy<RenderingEvent>,
    dispatcher: RenderDispatcher,
    #[cfg(feature = "devtools")]
    devtools_connection: DevtoolsWorldConnection,
}

fn init_world(world: &mut World, to_ecs: MainToEcs) {
    to_ecs.renderer_proxy.attach_to_ecs(world);
    to_ecs.dispatcher.attach_to_ecs(world, WINDOW_SIZE);

    InputHolder::new().attach_to_ecs(world);
    FreeCamera::new().attach_to_ecs(world);

    setup_assets_system(world, to_ecs.hub);
    setup_maps_system(world);
    setup_fullscreen_system(world);

    #[cfg(feature = "devtools")]
    {
        use crate::world::devtools::setup_devtools_system;
        setup_devtools_system(world, to_ecs.devtools_connection);
    }

    world.add_handler(escape_handler);
    world.add_handler(map_app_icon_handler);
}

// 'Make Zaebis' function
pub fn run_dawn<PH>(sync: WorldSyncMode, bi: BuildInfo, panic_hook: PH)
where
    PH: Fn(&PanicHookInfo) + Send + Sync + 'static,
{
    START_TIME.set(Instant::now()).ok();

    print_build_info(&bi);

    info!("Starting Dawn with sync mode: {:?}", sync);

    // We forced to do this here, because Bindings must be initialized passed to
    // the renderer that is created below. As well as the UI streamer.
    let mut hub = AssetHub::new();

    // Create window configuration
    let window_config = WindowConfig {
        title: format!("Dawn v{} - {}", bi.crate_info.version, bi.profile),
        decorations: true,
        icon: None,
        fullscreen: false,
        cursor: Some(Cursor::Icon(CursorIcon::Crosshair)),
        dimensions: WINDOW_SIZE,
        resizable: true,
        synchronization: match sync {
            WorldSyncMode::FixedTickRate(tps) => {
                // I think there's a better places to put this...
                let panic_hook = Box::leak(Box::new(panic_hook));
                panic::set_hook(Box::new(|info| {
                    panic_hook(info);
                }));

                None
            }
            WorldSyncMode::SynchronizedWithMonitor => {
                let before_frame = Rendezvous::new(2);
                let after_frame = Rendezvous::new(2);

                {
                    // We need to leak the rendezvous points to make sure they
                    // live for the entire duration of the program.
                    let before_frame = Box::leak(Box::new(before_frame.clone()));
                    let after_frame = Box::leak(Box::new(after_frame.clone()));
                    let panic_hook = Box::leak(Box::new(panic_hook));
                    panic::set_hook(Box::new(|info| {
                        panic_hook(info);

                        // TODO: Maybe move this to the library side?
                        // In case of a panic, we want to make sure that both threads can exit cleanly.
                        // So we signal both rendezvous points to avoid deadlocks.
                        before_frame.unlock();
                        after_frame.unlock();
                    }));
                }

                Some(RendererSynchronization {
                    before_frame: before_frame.clone(),
                    after_frame: after_frame.clone(),
                })
            }
        },
    };

    let backend_config = RendererConfig {
        shader_factory_binding: Some(hub.get_factory_biding(AssetType::Shader)),
        texture_factory_binding: Some(hub.get_factory_biding(AssetType::Texture)),
        mesh_factory_binding: Some(hub.get_factory_biding(AssetType::Mesh)),
        material_factory_binding: Some(hub.get_factory_biding(AssetType::Material)),
        font_factory_binding: Some(hub.get_factory_biding(AssetType::Font)),
    };

    #[cfg(feature = "devtools")]
    let (renderer_connection, world_connection) = devtools_bridge();

    // Construct the renderer
    // No rendering will happen until we call `run` on the renderer.
    // The renderer will run on the main thread, while the world loop
    let param = SetupRenderingParameters {
        #[cfg(feature = "devtools")]
        connection: renderer_connection,
        bi,
    };
    let (dispatcher, custom_renderer) = setup_rendering(param);
    let (renderer, proxy) =
        Renderer::new_with_monitoring(window_config.clone(), backend_config, custom_renderer)
            .unwrap();

    // Run the world loop
    // This will spawn a new thread that runs the world loop.
    // The main thread will run the renderer loop.
    let to_ecs = MainToEcs {
        hub,
        renderer_proxy: proxy,
        #[cfg(feature = "devtools")]
        devtools_connection: world_connection,
        dispatcher,
    };
    let world_loop = match sync {
        WorldSyncMode::FixedTickRate(tps) => {
            WorldLoopProxy::new_unsynchronized_with_monitoring(tps as f32, |w| {
                Ok(init_world(w, to_ecs))
            })
        }
        WorldSyncMode::SynchronizedWithMonitor => {
            let synchronization = window_config.synchronization.unwrap();
            WorldLoopProxy::new_synchronized_with_monitoring(
                synchronization.before_frame,
                synchronization.after_frame,
                |w| Ok(init_world(w, to_ecs)),
            )
        }
    }
    .unwrap();

    // Start the rendering loop
    // This will block the main thread until the window is closed.
    renderer.run();
    info!("Renderer loop has exited");

    // Drop the world loop first to make sure it exits cleanly.
    drop(world_loop);
    info!("World loop has exited");
}
