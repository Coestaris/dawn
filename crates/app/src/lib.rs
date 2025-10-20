pub mod assets;
#[cfg(feature = "devtools")]
pub mod devtools;
pub mod logging;
pub mod rendering;
pub mod world;

use crate::assets::reader::ReaderBackend;
#[cfg(feature = "devtools")]
use crate::devtools::devtools_bridge;
use crate::logging::{print_build_info, START_TIME};
use crate::rendering::preprocessor::shader_defines;
use crate::rendering::{RendererBuilder, SetupRenderingParameters};
use crate::world::{init_world, MainToEcs};
use build_info::BuildInfo;
use dawn_assets::hub::AssetHub;
use dawn_assets::AssetType;
use dawn_graphics::renderer::{
    Renderer, RendererConfig, RendererSynchronization, WindowConfig,
};
use dawn_util::rendezvous::Rendezvous;
use glam::UVec2;
use log::info;
use std::panic;
use std::panic::PanicHookInfo;
use std::sync::Arc;
use web_time::Instant;
use winit::window::{Cursor, CursorIcon};

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum WorldSyncMode {
    SynchronizedWithMonitor,
    FixedTickRate(usize),
}

pub(crate) static WINDOW_SIZE: UVec2 = UVec2::new(1280, 720);

// 'Make Zaebis' function
pub fn run_dawn<PH>(
    reader_backend: Arc<dyn ReaderBackend>,
    sync: WorldSyncMode,
    bi: BuildInfo,
    panic_hook: PH,
) where
    PH: Fn(&PanicHookInfo) + Send + Sync + 'static,
{
    START_TIME.set(Instant::now()).ok();

    print_build_info(&bi);

    info!("Starting Dawn with sync mode: {:?}", sync);

    // Create window configuration
    let window_config = WindowConfig {
        title: format!("Dawn v{} - {}", bi.crate_info.version, bi.profile),
        decorations: true,
        icon: None,
        fullscreen: false,
        cursor: Some(Cursor::Icon(CursorIcon::Crosshair)),
        dimensions: WINDOW_SIZE,
        resizable: true,
        #[cfg(feature = "threading")]
        synchronization: match sync {
            WorldSyncMode::FixedTickRate(_) => None,
            WorldSyncMode::SynchronizedWithMonitor => {
                let before_frame = Rendezvous::new(2);
                let after_frame = Rendezvous::new(2);

                Some(RendererSynchronization {
                    before_frame: before_frame.clone(),
                    after_frame: after_frame.clone(),
                })
            }
        },
        #[cfg(not(feature = "threading"))]
        synchronization: None,
    };

    // Setup panic hook
    if let Some(sync) = &window_config.synchronization {
        // We need to leak the rendezvous points to make sure they
        // live for the entire duration of the program.
        let before_frame = Box::leak(Box::new(sync.before_frame.clone()));
        let after_frame = Box::leak(Box::new(sync.after_frame.clone()));
        let panic_hook = Box::leak(Box::new(panic_hook));
        panic::set_hook(Box::new(|info| {
            panic_hook(info);
            // In case of a panic, we want to make sure that both threads can exit cleanly.
            // So we signal both rendezvous points to avoid deadlocks.
            before_frame.unlock();
            after_frame.unlock();
        }));
    } else {
        let panic_hook = Box::leak(Box::new(panic_hook));
        panic::set_hook(Box::new(|info| {
            panic_hook(info);
        }))
    }

    let mut hub = AssetHub::new();

    let backend_config = RendererConfig {
        shader_defines: Arc::new(shader_defines),
        shader_factory_binding: Some(hub.get_factory_biding(AssetType::Shader)),
        texture2d_factory_binding: Some(hub.get_factory_biding(AssetType::Texture2D)),
        texture_cube_factory_binding: Some(hub.get_factory_biding(AssetType::TextureCube)),
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
        reader_backend: reader_backend.clone(),
        bi,
    };

    let builder = RendererBuilder::new();
    let renderer_dispatcher = builder.build_dispatcher();
    let custom_renderer = builder.build_renderer(param);

    let (renderer, proxy) = {
        #[cfg(feature = "devtools")]
        {
            Renderer::new_with_monitoring(window_config.clone(), backend_config, custom_renderer)
                .unwrap()
        }
        #[cfg(not(feature = "devtools"))]
        {
            Renderer::new(window_config.clone(), backend_config, custom_renderer).unwrap()
        }
    };

    // Run the world loop
    // This will spawn a new thread that runs the world loop.
    // The main thread will run the renderer loop.
    let to_ecs = MainToEcs {
        reader_backend: reader_backend.clone(),
        hub,
        renderer_proxy: proxy,
        #[cfg(feature = "devtools")]
        devtools_connection: world_connection,
        dispatcher: renderer_dispatcher,
    };
    #[cfg(feature = "threading")]
    let _world_loop = match sync {
        WorldSyncMode::FixedTickRate(tps) => {
            use dawn_ecs::world::threading::WorldLoopProxy;
            #[cfg(feature = "devtools")]
            {
                WorldLoopProxy::new_unsynchronized_with_monitoring(tps as f32, |w| {
                    Ok(init_world(w, to_ecs))
                })
            }
            #[cfg(not(feature = "devtools"))]
            {
                WorldLoopProxy::new_unsynchronized(tps as f32, |w| Ok(crate::init_world(w, to_ecs)))
            }
        }
        WorldSyncMode::SynchronizedWithMonitor => {
            use dawn_ecs::world::threading::WorldLoopProxy;
            let synchronization = window_config.synchronization.clone().unwrap();
            #[cfg(feature = "devtools")]
            {
                WorldLoopProxy::new_synchronized_with_monitoring(
                    synchronization.before_frame,
                    synchronization.after_frame,
                    |w| Ok(init_world(w, to_ecs)),
                )
            }
            #[cfg(not(feature = "devtools"))]
            {
                WorldLoopProxy::new_synchronized(
                    synchronization.before_frame,
                    synchronization.after_frame,
                    |w| Ok(init_world(w, to_ecs)),
                )
            }
        }
    }
    .unwrap();
    #[cfg(not(feature = "threading"))]
    let mut _world_loop = WorldLoop::new_with_monitoring(|w| Ok(init_world(w, to_ecs))).unwrap();

    // Start the rendering loop
    // This will block the main thread until the window is closed.
    #[cfg(feature = "threading")]
    renderer.run(Box::new(move || true));
    #[cfg(not(feature = "threading"))]
    renderer.run(Box::new(move || match _world_loop.tick() {
        WorldLoopTickResult::Continue => true,
        WorldLoopTickResult::Exit => false,
    }));
    info!("Renderer loop has exited");

    // Drop the world loop first to make sure it exits cleanly.
    info!("World loop has exited");
}
