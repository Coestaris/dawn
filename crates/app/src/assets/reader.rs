use crate::logging::format_system_time;
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use dawn_dac::Manifest;
use log::info;

pub trait ReaderBackend: Send + Sync {
    fn enumerate(&self) -> Result<Manifest, anyhow::Error>;
    fn load(&self, aid: AssetID) -> Result<IRAsset, anyhow::Error>;
}

#[rustfmt::skip]
fn log_manifest(manifest: &Manifest) {
    info!("DAC Manifest:");
    info!("  Version: {}", manifest.version.as_ref().map_or("unknown".to_string(), |v| v.to_string()));
    info!("  Author: {}",  manifest.author.as_ref().map_or("unknown".to_string(), |v| v.to_string()));
    info!("  Description: {}", manifest.description.as_ref().map_or("unknown".to_string(), |v| v.to_string()));
    info!("  License: {}", manifest.license.as_ref().map_or("unknown".to_string(), |v| v.to_string()));
    info!("  Created: {}", format_system_time(manifest.created).unwrap_or("unknown".to_string()));
    info!("  Tool: {} (version {})", manifest.tool, manifest.tool_version);
    info!("  Assets: {}", manifest.headers.len());
}

#[cfg(feature = "threading")]
mod reader_impl {
    use crate::assets::reader::{log_manifest, ReaderBackend};
    use dawn_assets::ir::IRAsset;
    use dawn_assets::reader::{BasicReader, ReaderBinding};
    use dawn_assets::{AssetHeader, AssetID};
    use evenio::component::Component;
    use evenio::world::World;
    use log::{debug, info};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::thread::{Builder, JoinHandle};

    #[derive(Component)]
    pub struct Reader {
        handle: Option<JoinHandle<()>>,
        stop_signal: Arc<AtomicBool>,
    }

    impl Drop for Reader {
        fn drop(&mut self) {
            debug!("Stopping asset reader thread");
            self.stop_signal
                .store(true, std::sync::atomic::Ordering::Relaxed);
            if let Some(handle) = self.handle.take() {
                handle.join().unwrap();
            }
        }
    }

    impl Reader {
        pub(crate) fn new(backend: Arc<dyn ReaderBackend>, binding: ReaderBinding) -> Reader {
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
                                let res = Self::enumerate(backend.clone())?;
                                Ok(res)
                            },
                            |aid| {
                                let res = Self::load(backend.clone(), aid)?;
                                Ok(res)
                            },
                            web_time::Duration::from_millis(100),
                        );
                    }
                    debug!("Asset reader thread stopped")
                })
                .unwrap();

            Reader {
                handle: Some(handle),
                stop_signal,
            }
        }

        pub(crate) fn attach_to_ecs(self, world: &mut World) {
            let entity = world.spawn();
            world.insert(entity, self);
        }

        fn enumerate(backend: Arc<dyn ReaderBackend>) -> Result<Vec<AssetHeader>, anyhow::Error> {
            info!("Enumerating assets");
            let manifest = backend.enumerate()?;
            log_manifest(&manifest);
            Ok(manifest.headers)
        }

        fn load(backend: Arc<dyn ReaderBackend>, aid: AssetID) -> Result<IRAsset, anyhow::Error> {
            let asset = backend.load(aid)?;
            Ok(asset)
        }
    }
}

#[cfg(not(feature = "threading"))]
mod reader_impl {
    use crate::assets::reader::ReaderBackend;
    use dawn_assets::reader::{BasicReader, ReaderBinding};
    use dawn_ecs::events::TickEvent;
    use evenio::component::Component;
    use evenio::event::Receiver;
    use evenio::fetch::Single;
    use evenio::world::World;
    use std::sync::Arc;

    #[derive(Component)]
    pub struct Reader {
        backend: Arc<dyn ReaderBackend>,
        basic_reader: BasicReader,
    }

    impl Reader {
        pub(crate) fn new(backend: Arc<dyn ReaderBackend>, binding: ReaderBinding) -> Reader {
            let mut reader = BasicReader::new();
            reader.bind(binding);
            Reader {
                backend,
                basic_reader: reader,
            }
        }

        pub(crate) fn attach_to_ecs(self, world: &mut World) {
            fn tick_handler(_: Receiver<TickEvent>, reader: Single<&mut Reader>) {
                reader.basic_reader.process_events(
                    || {
                        let res = reader.backend.enumerate()?;
                        Ok(res.headers)
                    },
                    |aid| {
                        let res = reader.backend.load(aid)?;
                        Ok(res)
                    },
                    std::time::Duration::ZERO,
                );
            }

            let entity = world.spawn();
            world.add_handler(tick_handler);
            world.insert(entity, self);
        }
    }
}

pub use reader_impl::*;
