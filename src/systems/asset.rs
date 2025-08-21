use crate::logging::format_system_time;
use dawn_assets::factory::FactoryBinding;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::ir::IRAsset;
use dawn_assets::reader::AssetReader;
use dawn_assets::{AssetHeader, AssetID, AssetType};
use dawn_ecs::StopEventLoop;
use dawn_yarc::Manifest;
use evenio::event::{Receiver, Sender};
use evenio::prelude::World;
use log::{debug, error, info};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

pub struct FactoryBindings {
    pub shader: FactoryBinding,
    pub texture: FactoryBinding,
    pub mesh: FactoryBinding,
    pub material: FactoryBinding,
}

fn assets_failed_handler(r: Receiver<AssetHubEvent>, mut sender: Sender<StopEventLoop>) {
    match r.event {
        AssetHubEvent::QueryCompleted(_, false) => {
            error!("Asset query failed, stopping the event loop");
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
    let start = Instant::now();
    let mut hub = AssetHub::new(Reader).unwrap();
    info!("Asset hub created in {} ms", start.elapsed().as_millis());

    hub.query_load("barrel".into()).unwrap();
    hub.query_load("geometry".into()).unwrap();

    // Unlike other factories, shader and texture assets are
    // managed directly by the renderer, instead of processing assets
    // in the main loop (via ECS).
    let bindings = FactoryBindings {
        shader: hub.create_factory_biding(AssetType::Shader),
        texture: hub.create_factory_biding(AssetType::Texture),
        mesh: hub.create_factory_biding(AssetType::Mesh),
        material: hub.create_factory_biding(AssetType::Material),
    };

    hub.attach_to_ecs(world);
    world.add_handler(assets_failed_handler);

    bindings
}
