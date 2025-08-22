use crate::logging::format_system_time;
use dawn_assets::factory::FactoryBinding;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::ir::IRAsset;
use dawn_assets::reader::{BasicReader, ReaderBinding};
use dawn_assets::requests::{AssetRequest, AssetRequestQuery};
use dawn_assets::{AssetHeader, AssetID, AssetType};
use dawn_ecs::StopEventLoop;
use dawn_yarc::manifest::Manifest;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::prelude::World;
use log::{debug, error, info};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use std::time::Duration;

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
        let mut basic_reader = BasicReader::new();
        basic_reader.bind(binding);

        let stop_signal = Arc::new(AtomicBool::new(false));
        let thread_stop_signal = stop_signal.clone();
        let handle = Builder::new()
            .name("yarcreader".into())
            .spawn(move || {
                info!("Asset reader thread started");
                while !thread_stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                    basic_reader.process_events(
                        || Self::enumerate(),
                        |aid| Self::load(aid),
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

    fn yarc_path() -> PathBuf {
        PathBuf::from(env!("YARC_FILE"))
    }

    fn enumerate() -> Result<Vec<AssetHeader>, String> {
        info!("Enumerating assets");
        let manifest = dawn_yarc::reader::read_manifest(Self::yarc_path())
            .map_err(|e| format!("Failed to read asset manifest: {}", e))?;

        #[rustfmt::skip]
            fn log(manifest: &Manifest) {
                debug!("> Version: {}", manifest.version.as_ref().unwrap_or(&"unknown".to_string()));
                debug!("> Author: {}", manifest.author.as_ref().unwrap_or(&"unknown".to_string()));
                debug!("> Description: {}", manifest.description.as_ref().unwrap_or(&"unknown".to_string()));
                debug!("> License: {}", manifest.license.as_ref().unwrap_or(&"unknown".to_string()));
                debug!("> Created: {}", format_system_time(manifest.created).unwrap());
                debug!("> Tool: {} (version {})", manifest.tool, manifest.tool_version);
                debug!("> Assets: {}", manifest.headers.len());
            }
        log(&manifest);
        Ok(manifest.headers)
    }

    fn load(aid: AssetID) -> Result<IRAsset, String> {
        dawn_yarc::reader::read(Self::yarc_path(), aid.clone())
            .map_err(|e| format!("Failed to read asset {}: {}", aid, e))
    }
}

pub struct FactoryBindings {
    pub shader: FactoryBinding,
    pub texture: FactoryBinding,
    pub mesh: FactoryBinding,
    pub material: FactoryBinding,
}

fn assets_failed_handler(r: Receiver<AssetHubEvent>, mut sender: Sender<StopEventLoop>) {
    match r.event {
        AssetHubEvent::RequestCompleted(request, Err(message)) => {
            error!("Asset {} request failed: {}", request, message);
            sender.send(StopEventLoop);
        }
        AssetHubEvent::AssetFailed(aid, error) => {
            let error = match error {
                Some(e) => e.to_string(),
                None => "Unknown error".to_string(),
            };
            error!("Aborting due to asset load failure: {}: {}", aid, error);
            sender.send(StopEventLoop);
        }
        _ => {}
    }
}

pub fn setup_assets_system(world: &mut World) -> FactoryBindings {
    let mut hub = AssetHub::new().unwrap();
    let reader = ReaderHandle::new(hub.get_read_binding());
    reader.attach_to_ecs(world);

    hub.request(AssetRequest::Enumerate);
    hub.request(AssetRequest::Load(AssetRequestQuery::ByID("barrel".into())));
    hub.request(AssetRequest::Load(AssetRequestQuery::ByID(
        "geometry".into(),
    )));

    // Unlike other factories, shader and texture assets are
    // managed directly by the renderer, instead of processing assets
    // in the main loop (via ECS).
    let bindings = FactoryBindings {
        shader: hub.get_factory_biding(AssetType::Shader),
        texture: hub.get_factory_biding(AssetType::Texture),
        mesh: hub.get_factory_biding(AssetType::Mesh),
        material: hub.get_factory_biding(AssetType::Material),
    };

    hub.attach_to_ecs(world);
    world.add_handler(assets_failed_handler);

    bindings
}
