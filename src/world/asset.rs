use crate::logging::format_system_time;
use crate::world::asset_swap::load_assets;
use crate::world::dictionaries::DictionaryAssetFactory;
use dawn_assets::factory::FactoryBinding;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::ir::IRAsset;
use dawn_assets::reader::{BasicReader, ReaderBinding};
use dawn_assets::{AssetHeader, AssetID, AssetType};
use dawn_dac::reader::{read_asset, read_manifest};
use dawn_dac::{ContainerError, Manifest};
use dawn_ecs::events::ExitEvent;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::prelude::World;
use log::{debug, info};
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

pub struct FactoryBindings {
    pub shader: FactoryBinding,
    pub texture: FactoryBinding,
    pub mesh: FactoryBinding,
    pub material: FactoryBinding,
    pub font: FactoryBinding,
}

fn assets_failed_handler(r: Receiver<AssetHubEvent>, sender: Sender<ExitEvent>) {
    match r.event {
        AssetHubEvent::RequestFinished(request, Err(message)) => {
            panic!("Asset Request Failed {:?}: {:?}", request, message);
        }
        _ => {}
    }
}

pub fn setup_assets_system(world: &mut World, mut hub: AssetHub) {
    let reader = ReaderHandle::new(hub.get_read_binding());
    reader.attach_to_ecs(world);

    load_assets(&mut hub);

    let mut dictionary_factory = DictionaryAssetFactory::new();
    dictionary_factory.bind(hub.get_factory_biding(AssetType::Dictionary));
    dictionary_factory.attach_to_ecs(world);

    hub.attach_to_ecs(world);
    world.add_handler(assets_failed_handler);
}
