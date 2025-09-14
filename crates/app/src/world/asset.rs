use crate::logging::format_system_time;
use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::RenderingEvent;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::ir::IRAsset;
use dawn_assets::reader::{BasicReader, ReaderBinding};
use dawn_assets::requests::{AssetRequest, AssetRequestID, AssetRequestQuery};
use dawn_assets::{AssetHeader, AssetID, AssetType};
use dawn_dac::reader::{read_asset, read_manifest};
use dawn_dac::{ContainerError, Manifest};
use dawn_ecs::events::{ExitEvent, TickEvent};
use dawn_graphics::ecs::{InvalidateRendererCache, ObjectMesh};
use dawn_graphics::passes::events::RenderPassEvent;
use evenio::component::Component;
use evenio::entity::EntityId;
use evenio::event::{GlobalEvent, Insert, Receiver, Remove, Sender, Spawn};
use evenio::fetch::{Fetcher, Single};
use evenio::prelude::World;
use log::{debug, info};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use std::time::Duration;
use crate::world::assets::blob::BlobAssetFactory;
use crate::world::assets::dict::DictionaryAssetFactory;

pub const CURRENT_MAP: &str = "map1";

pub const APPLICATION_ICON_BLOB_ID: &str = "icon_blob";
pub const LIGHT_TEXTURE: &str = "light_texture";

pub const GEOMETRY_SHADER: &str = "geometry_shader";
pub const LINE_SHADER: &str = "line_shader";
pub const BILLBOARD_SHADER: &str = "billboard_shader";
pub const LIGHTING_SHADER: &str = "lighting_shader";
pub const POSTPROCESS_SHADER: &str = "postprocess_shader";

#[derive(Component)]
struct ReaderHandle {
    handle: Option<JoinHandle<()>>,
    stop_signal: Arc<AtomicBool>,
}

impl Drop for ReaderHandle {
    fn drop(&mut self) {
        debug!("Stopping asset reader thread");
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }
}

impl ReaderHandle {
    fn new(binding: ReaderBinding) -> ReaderHandle {
        info!("DAC path: {:?}", Self::dac_path());
        let mut basic_reader = BasicReader::new();
        basic_reader.bind(binding);

        let stop_signal = Arc::new(AtomicBool::new(false));
        let thread_stop_signal = stop_signal.clone();
        let handle = Builder::new()
            .name("dac_reader".into())
            .spawn(move || {
                info!("Asset reader thread started");
                while !thread_stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                    basic_reader.process_events(
                        || {
                            let res = Self::enumerate()?;
                            Ok(res)
                        },
                        |aid| {
                            let res = Self::load(aid)?;
                            Ok(res)
                        },
                        Duration::from_millis(100),
                    );
                }
                debug!("Asset reader thread stopped")
            })
            .unwrap();

        ReaderHandle {
            handle: Some(handle),
            stop_signal,
        }
    }

    fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
    }

    fn dac_path() -> PathBuf {
        // Try to find file with the same name in the current directory
        let path = std::env::current_dir().unwrap().join("assets.dac");
        if path.exists() {
            path
        } else {
            let exe_dir = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf();
            let path = exe_dir.join("assets.dac");
            if path.exists() {
                path
            } else {
                panic!("DAC file not found. Please ensure 'assets.dac' is present in the current directory or the executable directory.");
            }
        }
    }

    fn enumerate() -> Result<Vec<AssetHeader>, ContainerError> {
        info!("Enumerating assets");
        let file = std::fs::File::open(Self::dac_path()).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let manifest = read_manifest(&mut reader)?;

        #[rustfmt::skip]
        fn log(manifest: &Manifest) {
            info!("DAC Manifest:");
            info!("  Version: {}", manifest.version.as_ref().map_or("unknown".to_string(), |v| v.to_string()));
            info!("  Author: {}",  manifest.author.as_ref().map_or("unknown".to_string(), |v| v.to_string()));
            info!("  Description: {}", manifest.description.as_ref().map_or("unknown".to_string(), |v| v.to_string()));
            info!("  License: {}", manifest.license.as_ref().map_or("unknown".to_string(), |v| v.to_string()));
            info!("  Created: {}", format_system_time(manifest.created).unwrap_or("unknown".to_string()));
            info!("  Tool: {} (version {})", manifest.tool, manifest.tool_version);
            info!("  Assets: {}", manifest.headers.len());
        }

        log(&manifest);
        Ok(manifest.headers)
    }

    fn load(aid: AssetID) -> Result<IRAsset, ContainerError> {
        let file = std::fs::File::open(Self::dac_path()).unwrap();
        let mut reader = std::io::BufReader::new(file);
        read_asset(&mut reader, aid.clone())
    }
}

fn assets_failed_handler(r: Receiver<AssetHubEvent>, sender: Sender<ExitEvent>) {
    match r.event {
        AssetHubEvent::RequestFinished(request, Err(message)) => {
            panic!("Asset Request Failed {:?}: {:?}", request, message);
        }
        _ => {}
    }
}

#[derive(Component)]
struct Timer {
    and_then: AndThen,
    ticks: usize,
}

#[derive(Component)]
struct FreeAllAssetsRequest(AssetRequestID, pub AndThen);

#[derive(GlobalEvent)]
pub struct DropAllAssetsEvent(pub AndThen);

#[derive(Clone, Debug)]
pub enum AndThen {
    StopWorldLoop,
    ReloadAssets,
}

#[derive(GlobalEvent)]
struct AllAssetsDroppedEvent(pub AndThen);

fn drop_all_assets_in_renderer_handler(
    _: Receiver<DropAllAssetsEvent>,
    mut sender: Sender<InvalidateRendererCache>,
) {
    sender.send(InvalidateRendererCache);
}

fn drop_all_assets_in_pipeline_handler(
    _: Receiver<DropAllAssetsEvent>,
    dispatcher: Single<&RenderDispatcher>,
    sender: Sender<RenderPassEvent<RenderingEvent>>,
) {
    dispatcher.dispatch_drop_assets(sender);
}

fn drop_all_assets_in_world_handler(
    r: Receiver<DropAllAssetsEvent>,
    f: Fetcher<(EntityId, &ObjectMesh)>,
    mut sender: Sender<(Remove<ObjectMesh>, Spawn, Insert<Timer>)>,
) {
    info!("DropAllAssetsEvent received: {:?}", r.event.0);

    // Remove all assets from the ECS
    for (entity, _) in f.iter() {
        sender.remove::<ObjectMesh>(entity);
    }

    // Assuming that the rendering thread is not throttled, so a logic update
    // period is the same as the rendering period.
    // It takes some time to drop assets:
    //    - Maximum of 3 frames to pass the event to the renderer
    //    - Maximum of 3 frames to empty the triple buffer used
    //      for Renderables streaming
    // You can experiment with this value to see how it affects the delay
    // of asset reload.
    const TIMER_INTERVAL: usize = 5; // In Frames

    // Spawn a timer to remove the assets when they are all dropped
    let id = sender.spawn();
    sender.insert(
        id,
        Timer {
            and_then: r.event.0.clone(),
            ticks: TIMER_INTERVAL,
        },
    );
}

fn timer_handler(
    _: Receiver<TickEvent>,
    mut f: Fetcher<(EntityId, &mut Timer)>,
    mut sender: Sender<(Remove<Timer>, AllAssetsDroppedEvent)>,
) {
    for timer in f.iter_mut() {
        if timer.1.ticks == 0 {
            info!("All assets dropped, removing timer");
            sender.remove::<Timer>(timer.0);
            sender.send(AllAssetsDroppedEvent(timer.1.and_then.clone()));
        } else {
            timer.1.ticks -= 1;
        }
    }
}

fn free_assets_handler(
    r: Receiver<AllAssetsDroppedEvent>,
    mut hub: Single<&mut AssetHub>,
    mut sender: Sender<(Spawn, Insert<FreeAllAssetsRequest>)>,
) {
    let request = sender.spawn();
    sender.insert(
        request,
        FreeAllAssetsRequest(free_assets(*hub), r.event.0.clone()),
    );
}

fn request_finished(
    r: Receiver<AssetHubEvent>,
    f: Fetcher<(EntityId, &FreeAllAssetsRequest)>,
    mut hub: Single<&mut AssetHub>,
    mut sender: Sender<(ExitEvent, Remove<FreeAllAssetsRequest>)>,
) {
    let (rid, and_then) = match f.iter().next() {
        Some((_, req)) => (req.0, req.1.clone()),
        None => return,
    };
    if let AssetHubEvent::RequestFinished(id, Ok(())) = r.event {
        if *id == rid {
            match and_then {
                AndThen::ReloadAssets => {
                    info!("Free all assets request finished, reloading assets");
                    load_assets(*hub);
                }
                AndThen::StopWorldLoop => {
                    // The request to free all assets is finished
                    // The actual removal of assets from ECS is done in the timer handler
                    info!("Free all assets request finished");
                    sender.send(ExitEvent);
                }
            }
            sender.remove::<FreeAllAssetsRequest>(f.iter().next().unwrap().0);
        }
    }
}

fn load_assets(hub: &mut AssetHub) {
    hub.request(AssetRequest::Enumerate);
    hub.request(AssetRequest::Load(AssetRequestQuery::ByType(
        AssetType::Blob,
    )));
    hub.request(AssetRequest::Load(AssetRequestQuery::ByType(
        AssetType::Shader,
    )));
    hub.request(AssetRequest::LoadNoDeps(AssetRequestQuery::ByType(
        AssetType::Dictionary,
    )));
    hub.request(AssetRequest::Load(AssetRequestQuery::All));
}

fn free_assets(hub: &mut AssetHub) -> AssetRequestID {
    hub.request(AssetRequest::FreeNoDeps(AssetRequestQuery::ByType(
        AssetType::Mesh,
    )));
    // Materials are holding the textures. Free them first.
    // Hoping that no one else is using them otherwise it's circular reference.
    //   Textures are stored in Material so cannot be freed first
    //   Materials depends on Textures so cannot be freed first
    hub.request(AssetRequest::FreeNoDeps(AssetRequestQuery::ByType(
        AssetType::Material,
    )));
    // Same with Fonts
    hub.request(AssetRequest::FreeNoDeps(AssetRequestQuery::ByType(
        AssetType::Font,
    )));
    hub.request(AssetRequest::Free(AssetRequestQuery::All))
}

pub fn setup_assets_system(world: &mut World, mut hub: AssetHub) {
    // Request initial assets
    load_assets(&mut hub);

    // Setup the asset reader thread
    // It will read the DAC file and load assets into the AssetHub
    let reader = ReaderHandle::new(hub.get_read_binding());
    reader.attach_to_ecs(world);

    // Setup the dictionary factory. It's quite unique because it the only
    // one that defined on our side.
    let mut dictionary_factory = DictionaryAssetFactory::new();
    dictionary_factory.bind(hub.get_factory_biding(AssetType::Dictionary));
    dictionary_factory.attach_to_ecs(world);
    let mut blob_factory = BlobAssetFactory::new();
    blob_factory.bind(hub.get_factory_biding(AssetType::Blob));
    blob_factory.attach_to_ecs(world);

    // Move the AssetHub into the ECS
    hub.attach_to_ecs(world);
    world.add_handler(assets_failed_handler);
    // Former 'asset swap' system
    // First we wait for DropAllAssets event
    world.add_handler(drop_all_assets_in_renderer_handler);
    world.add_handler(drop_all_assets_in_world_handler);
    world.add_handler(drop_all_assets_in_pipeline_handler);
    // Then we wait for the timer to finish
    world.add_handler(timer_handler);
    // Then we request the AssetHub to free all assets
    world.add_handler(free_assets_handler);
    // After the AssetHub finished the request, we stop the main loop

    world.add_handler(request_finished);
}
