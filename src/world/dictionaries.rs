use std::fmt::Display;
use dawn_assets::factory::{BasicFactory, FactoryBinding};
use dawn_assets::ir::dictionary::IRDictionary;
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetCastable, AssetMemoryUsage};
use dawn_ecs::events::TickEvent;
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::Single;
use evenio::world::World;
use glam::{Quat, Vec3};
use log::warn;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapUID(usize);

impl Display for MapUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MapUID({})", self.0)
    }
}

impl MapUID {
    pub fn new() -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let uid = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self(uid)
    }
}

pub struct MapObject {
    pub uid: MapUID,
    pub location: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub mesh: String,
    pub components: Vec<String>,
}

impl MapObject {
    fn new() -> Self {
        Self {
            uid: MapUID::new(),
            location: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            mesh: String::new(),
            components: Vec::new(),
        }
    }
}

pub struct MapPointLight {
    pub uid: MapUID,
    pub location: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
    pub components: Vec<String>,
}

impl MapPointLight {
    fn new() -> Self {
        Self {
            uid: MapUID::new(),
            location: Vec3::ZERO,
            color: Vec3::ONE,
            intensity: 1.0,
            range: 100.0,
            components: Vec::new(),
        }
    }
}

pub struct Map {
    pub objects: Vec<MapObject>,
    pub point_lights: Vec<MapPointLight>,
}

pub enum DictionaryEntry {
    Map(Map),
    // Other asset types can be added here
}

impl DictionaryEntry {
    pub fn as_map(&self) -> Option<&Map> {
        if let DictionaryEntry::Map(map) = self {
            Some(map)
        } else {
            None
        }
    }
}

impl AssetCastable for DictionaryEntry {}

#[derive(Component)]
pub struct DictionaryAssetFactory {
    basic_factory: BasicFactory<DictionaryEntry>,
}

fn process_dictionary(ir: IRDictionary) -> anyhow::Result<(DictionaryEntry, AssetMemoryUsage)> {
    // Assume only map dictionaries for now
    // TODO: Error handling?
    let mut map = Map {
        objects: Vec::new(),
        point_lights: Vec::new(),
    };
    let dict = ir.entries.iter().next().unwrap().as_map().unwrap();
    if let Some(objects) = dict.get("Objects") {
        let objects = objects.as_array().unwrap();
        for entry in objects {
            let obj_map = entry.as_map().unwrap();
            let mut object = MapObject::new();
            for (key, value) in obj_map {
                match key.as_str() {
                    "Location" => {
                        object.location = value.as_vec3f().unwrap();
                    }
                    "Rotation" => {
                        // TODO: Handle Euler angles too
                    }
                    "Scale" => {
                        object.scale = value.as_vec3f().unwrap();
                    }
                    "Mesh" => {
                        object.mesh = value.as_string().unwrap().to_string();
                    }
                    "Components" => {
                        let comps = value.as_array().unwrap();
                        for comp in comps {
                            object
                                .components
                                .push(comp.as_string().unwrap().to_string());
                        }
                    }
                    _ => {
                        warn!("Unknown object key in dictionary: {}", key);
                    }
                }
            }

            map.objects.push(object);
        }
    }

    if let Some(lights) = dict.get("PointLights") {
        let lights = lights.as_array().unwrap();
        for light in lights {
            let light_map = light.as_map().unwrap();
            let mut point_light = MapPointLight::new();
            for (key, value) in light_map {
                match key.as_str() {
                    "Location" => {
                        point_light.location = value.as_vec3f().unwrap();
                    }
                    "Color" => {
                        point_light.color = value.as_vec3f().unwrap();
                    }
                    "Intensity" => {
                        point_light.intensity = value.as_f32().unwrap();
                    }
                    "Range" => {
                        point_light.range = value.as_f32().unwrap();
                    }
                    "Components" => {
                        let comps = value.as_array().unwrap();
                        for comp in comps {
                            point_light
                                .components
                                .push(comp.as_string().unwrap().to_string());
                        }
                    }
                    _ => {
                        warn!("Unknown point light key in dictionary: {}", key);
                    }
                }
            }

            map.point_lights.push(point_light);
        }
    }

    Ok((DictionaryEntry::Map(map), AssetMemoryUsage::new(0, 0)))
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
                        process_dictionary(dictionary)
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
