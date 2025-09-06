use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::{RenderingEvent, RenderingEventMask};
use crate::rendering::gbuffer::GBuffer;
use crate::rendering::passes::bounding_pass::BoundingPass;
use crate::rendering::passes::geometry_pass::GeometryPass;
use crate::rendering::passes::screen_pass::ScreenPass;
use crate::rendering::{pre_pipeline_construct, setup_rendering};
use crate::ui::{ui_bridge, UIWorldConnection};
use crate::world::asset::setup_assets_system;
use crate::world::exit::escape_handler;
use crate::world::fcam::FreeCamera;
use crate::world::fullscreen::setup_fullscreen_system;
use crate::world::input::InputHolder;
use crate::world::maps::setup_maps_system;
use crate::{logging, panic_hook, WorldSyncMode};
use dawn_assets::hub::AssetHub;
use dawn_assets::AssetType;
use dawn_ecs::world::WorldLoopProxy;
use dawn_graphics::construct_chain;
use dawn_graphics::passes::chain::{ChainCons, ChainNil};
use dawn_graphics::renderer::{
    Renderer, RendererConfig, RendererProxy, RendererSynchronization, WindowConfig,
};
use dawn_util::rendezvous::Rendezvous;
use evenio::prelude::World;
use glam::UVec2;
use log::info;
use std::panic;
use std::rc::Rc;
use triple_buffer::Input;
use winit::window::{Cursor, CursorIcon};
use crate::world::ui::setup_ui_system;

pub(crate) static WINDOW_SIZE: UVec2 = UVec2::new(1280, 720);

struct MainToEcs {
    hub: AssetHub,
    renderer_proxy: RendererProxy<RenderingEvent>,
    ui_connection: UIWorldConnection,
    dispatcher: RenderDispatcher,
}

fn init_world(world: &mut World, to_ecs: MainToEcs) {
    to_ecs.renderer_proxy.attach_to_ecs(world);
    to_ecs.dispatcher.attach_to_ecs(world, WINDOW_SIZE);

    InputHolder::new().attach_to_ecs(world);
    FreeCamera::new().attach_to_ecs(world);

    setup_assets_system(world, to_ecs.hub);
    setup_maps_system(world);
    setup_fullscreen_system(world);
    setup_ui_system(world, to_ecs.ui_connection);

    world.add_handler(escape_handler);
}

// 'Make Zaebis' function
pub fn run_dawn(sync: WorldSyncMode) {
    info!("Starting Dawn with sync mode: {:?}", sync);

    // We forced to do this here, because Bindings must be initialized passed to
    // the renderer that is created below. As well as the UI streamer.
    let mut hub = AssetHub::new();

    // Create window configuration
    let bi = logging::dawn_build_info();
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

    let (renderer_ui, world_ui) = ui_bridge();
    let backend_config = RendererConfig {
        shader_factory_binding: Some(hub.get_factory_biding(AssetType::Shader)),
        texture_factory_binding: Some(hub.get_factory_biding(AssetType::Texture)),
        mesh_factory_binding: Some(hub.get_factory_biding(AssetType::Mesh)),
        material_factory_binding: Some(hub.get_factory_biding(AssetType::Material)),
        font_factory_binding: Some(hub.get_factory_biding(AssetType::Font)),
    };

    // Construct the renderer
    // No rendering will happen until we call `run` on the renderer.
    // The renderer will run on the main thread, while the world loop
    let (dispatcher, custom_renderer) = setup_rendering(renderer_ui);
    let (renderer, proxy) =
        Renderer::new_with_monitoring(window_config.clone(), backend_config, custom_renderer)
            .unwrap();

    // Run the world loop
    // This will spawn a new thread that runs the world loop.
    // The main thread will run the renderer loop.
    let to_ecs = MainToEcs {
        hub,
        renderer_proxy: proxy,
        ui_connection: world_ui,
        dispatcher,
    };
    let _world_loop = match sync {
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
    drop(_world_loop);
    info!("World loop has exited");
}
