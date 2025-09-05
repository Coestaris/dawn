use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::{RenderingEvent, RenderingEventMask};
use crate::rendering::gbuffer::GBuffer;
use crate::rendering::passes::bounding_pass::BoundingPass;
use crate::rendering::passes::geometry_pass::GeometryPass;
use crate::rendering::passes::screen_pass::ScreenPass;
use crate::rendering::passes::ui_pass::UIPass;
use crate::rendering::pre_pipeline_construct;
use crate::world::asset::setup_assets_system;
use crate::world::exit::escape_handler;
use crate::world::fcam::FreeCamera;
use crate::world::input::InputHolder;
use crate::world::maps::setup_maps_system;
use crate::world::ui::{setup_ui_system, UICommand, UIReader};
use crate::{logging, panic_hook, WorldSyncMode, WORLD_SYNC_MODE};
use dawn_assets::hub::AssetHub;
use dawn_assets::AssetType;
use dawn_ecs::world::WorldLoopProxy;
use dawn_graphics::construct_chain;
use dawn_graphics::passes::chain::{ChainCons, ChainNil, RenderChain};
use dawn_graphics::passes::pipeline::RenderPipeline;
use dawn_graphics::renderer::{
    Renderer, RendererConfig, RendererProxy, RendererSynchronization, WindowConfig,
};
use dawn_util::rendezvous::Rendezvous;
use evenio::prelude::World;
use glam::UVec2;
use log::info;
use std::panic;
use std::rc::Rc;
use std::sync::Arc;
use triple_buffer::{triple_buffer, Input};
use winit::window::{Cursor, CursorIcon};
use crate::world::fullscreen::setup_fullscreen_system;

static WINDOW_SIZE: UVec2 = UVec2::new(1280, 720);

struct MainToEcs {
    hub: AssetHub,
    ui_writer: Input<Vec<UICommand>>,
    renderer_proxy: RendererProxy<RenderingEvent>,
    dispatcher: RenderDispatcher,
}

fn init_world(world: &mut World, to_ecs: MainToEcs) {
    to_ecs.renderer_proxy.attach_to_ecs(world);
    to_ecs.dispatcher.attach_to_ecs(world, WINDOW_SIZE);

    InputHolder::new().attach_to_ecs(world);
    FreeCamera::new().attach_to_ecs(world);

    setup_assets_system(world, to_ecs.hub);
    setup_maps_system(world);
    setup_ui_system(world, to_ecs.ui_writer);
    setup_fullscreen_system(world);

    world.add_handler(escape_handler);
}

// 'Make Zaebis' function
pub fn run_dawn(sync: WorldSyncMode) {
    info!("Starting Dawn with sync mode: {:?}", sync);

    // We forced to do this here, because Bindings must be initialized passed to
    // the renderer that is created below. As well as the UI streamer.
    let mut hub = AssetHub::new();
    let (ui_writer, ui_reader) = UIReader::bridge();

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

    let backend_config = RendererConfig {
        shader_factory_binding: Some(hub.get_factory_biding(AssetType::Shader)),
        texture_factory_binding: Some(hub.get_factory_biding(AssetType::Texture)),
        mesh_factory_binding: Some(hub.get_factory_biding(AssetType::Mesh)),
        material_factory_binding: Some(hub.get_factory_biding(AssetType::Material)),
        font_factory_binding: Some(hub.get_factory_biding(AssetType::Font)),
    };

    // Allocate the render pass IDs and select the events they will respond to.
    // This must be done before creating the renderer, because the passes
    // will need the IDs during their construction.
    let mut dispatcher = RenderDispatcher::new();
    let geometry_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::VIEW_UPDATED
            | RenderingEventMask::VIEWPORT_RESIZED
            | RenderingEventMask::PERSP_PROJECTION_UPDATED
            | RenderingEventMask::TOGGLE_WIREFRAME_MODE,
        "geometry_shader",
    );
    let bounding_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::VIEW_UPDATED
            | RenderingEventMask::VIEWPORT_RESIZED
            | RenderingEventMask::PERSP_PROJECTION_UPDATED
            | RenderingEventMask::TOGGLE_BOUNDING,
        "bounding_shader",
    );
    let ui_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::ORTHO_PROJECTION_UPDATED,
        "glyph_shader",
    );
    let screen_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::VIEWPORT_RESIZED,
        "screen_shader",
    );

    // Construct the renderer
    // No rendering will happen until we call `run` on the renderer.
    // The renderer will run on the main thread, while the world loop
    let (renderer, proxy) =
        Renderer::new_with_monitoring(window_config.clone(), backend_config, move |r| {
            pre_pipeline_construct(&r.gl);

            let gbuffer = Rc::new(GBuffer::new(&r.gl, WINDOW_SIZE));
            let geometry_pass = GeometryPass::new(&r.gl, geometry_id, gbuffer.clone());
            let bounding_pass = BoundingPass::new(&r.gl, bounding_id, gbuffer.clone());
            let ui_pass = UIPass::new(&r.gl, ui_id, ui_reader.clone());
            let screen_pass = ScreenPass::new(&r.gl, screen_id, gbuffer.clone());
            Ok(RenderPipeline::new(construct_chain!(
                geometry_pass,
                screen_pass,
                bounding_pass,
                ui_pass,
            )))
        })
        .unwrap();

    // Run the world loop
    // This will spawn a new thread that runs the world loop.
    // The main thread will run the renderer loop.
    let to_ecs = MainToEcs {
        hub,
        ui_writer,
        renderer_proxy: proxy,
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
