use crate::assets::map::{parse_entries, MapEntry};
use dawn_assets::factory::{BasicFactory, FactoryBinding};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetCastable, AssetMemoryUsage};
use dawn_ecs::events::TickEvent;
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::Single;
use evenio::prelude::World;
use web_time::Duration;

pub enum DictionaryEntry {
    Map(Vec<MapEntry>),
    // Other asset types can be added here
}

impl DictionaryEntry {
    pub fn as_map(&self) -> Option<&Vec<MapEntry>> {
        match self {
            DictionaryEntry::Map(map) => Some(map),
        }
    }
}

impl AssetCastable for DictionaryEntry {}

#[derive(Component)]
pub struct DictionaryAssetFactory {
    basic_factory: BasicFactory<DictionaryEntry>,
}

impl DictionaryAssetFactory {
    pub fn new() -> Self {
        Self {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        self.basic_factory.bind(binding);
    }

    pub fn attach_to_ecs(self, world: &mut World) {
        fn process_events_handler(
            _: Receiver<TickEvent>,
            factory: Single<&mut DictionaryAssetFactory>,
        ) {
            factory.0.basic_factory.process_events(
                |msg| {
                    if let IRAsset::Dictionary(dictionary) = msg.ir {
                        // Assume dictionary entries are of type Map for this example
                        Ok((
                            DictionaryEntry::Map(parse_entries(dictionary)?),
                            AssetMemoryUsage::new(0, 0),
                        ))
                    } else {
                        Err(anyhow::anyhow!("Expected Dictionary asset"))
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
