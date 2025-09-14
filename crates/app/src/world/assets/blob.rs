use dawn_assets::factory::{BasicFactory, FactoryBinding};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetCastable, AssetMemoryUsage};
use dawn_ecs::events::TickEvent;
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::Single;
use evenio::prelude::World;
use web_time::Duration;

pub struct Blob {
    pub data: Vec<u8>,
}

#[derive(Component)]
pub struct BlobAssetFactory {
    basic_factory: BasicFactory<Blob>,
}

impl AssetCastable for Blob {}

impl BlobAssetFactory {
    pub fn new() -> Self {
        Self {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        self.basic_factory.bind(binding);
    }

    pub fn attach_to_ecs(self, world: &mut World) {
        fn process_events_handler(_: Receiver<TickEvent>, factory: Single<&mut BlobAssetFactory>) {
            factory.0.basic_factory.process_events(
                |msg| {
                    if let IRAsset::Blob(data) = msg.ir {
                        let len = data.data.len();
                        Ok((Blob { data: data.data }, AssetMemoryUsage::new(len, 0)))
                    } else {
                        Err(anyhow::anyhow!("Expected Blob asset"))
                    }
                },
                |_| {},
                Duration::ZERO,
            );
        }

        world.add_handler(process_events_handler);

        let entity = world.spawn();
        world.insert(entity, self);
    }
}
