use dawn_assets::ir::dictionary::{IRDictionary, IRDictionaryEntry};
use glam::{Quat, Vec3};
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapEntryID(usize);

impl Display for MapEntryID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MapUID({})", self.0)
    }
}

impl MapEntryID {
    pub fn new() -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let uid = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self(uid)
    }
}

pub struct MapEntryMeta {
    pub id: MapEntryID,
    pub components: Vec<String>,
}

#[derive(Clone)]
pub enum MapEntryData {
    Mesh {
        location: Vec3,
        mesh: String,
        scale: Vec3,
        rotation: Quat,
    },
    PointLight {
        location: Vec3,
        color: Vec3,
        intensity: f32,
        linear_falloff: bool,
        range: f32,
        shadow: bool,
    },
    SunLight {
        direction: Vec3,
        color: Vec3,
        intensity: f32,
        ambient: f32,
        shadow: bool,
    },
    SpotLight {
        location: Vec3,
        direction: Vec3,
        color: Vec3,
        intensity: f32,
        range: f32,
        inner_cone_angle: f32,
        outer_cone_angle: f32,
        linear_falloff: bool,
        shadow: bool,
    },
}

pub struct MapEntry {
    pub meta: MapEntryMeta,
    pub data: MapEntryData,
}

fn extract<T>(
    kv: &HashMap<String, IRDictionaryEntry>,
    key: &str,
    extract: fn(&IRDictionaryEntry) -> Option<T>,
) -> Option<T> {
    kv.get(key).and_then(extract)
}

fn extract_vec3(kv: &HashMap<String, IRDictionaryEntry>, key: &str) -> Option<Vec3> {
    extract(kv, key, |entry| entry.as_vec3f())
}

fn extract_f32(kv: &HashMap<String, IRDictionaryEntry>, key: &str) -> Option<f32> {
    extract(kv, key, |entry| entry.as_f32())
}

fn extract_string(kv: &HashMap<String, IRDictionaryEntry>, key: &str) -> Option<String> {
    extract(kv, key, |entry| entry.as_string().map(|s| s.to_string()))
}

fn extract_bool(kv: &HashMap<String, IRDictionaryEntry>, key: &str) -> Option<bool> {
    extract(kv, key, |entry| entry.as_bool())
}

fn extract_string_vec(kv: &HashMap<String, IRDictionaryEntry>, key: &str) -> Option<Vec<String>> {
    extract(kv, key, |entry| {
        entry.as_array().map(|arr| {
            arr.iter()
                .filter_map(|e| e.as_string().map(|s| s.to_string()))
                .collect()
        })
    })
}

fn kv_to_object(kv: HashMap<String, IRDictionaryEntry>) -> anyhow::Result<MapEntry> {
    Ok(MapEntry {
        meta: MapEntryMeta {
            id: MapEntryID::new(),
            components: extract_string_vec(&kv, "Components").unwrap_or(vec![]),
        },
        data: MapEntryData::Mesh {
            location: extract_vec3(&kv, "Location").unwrap_or(Vec3::ZERO),
            mesh: extract_string(&kv, "Mesh").unwrap_or("".to_string()),
            scale: extract_vec3(&kv, "Scale").unwrap_or(Vec3::ONE),
            rotation: Quat::from_euler(
                glam::EulerRot::XYZ,
                extract_f32(&kv, "RotationX").unwrap_or(0.0).to_radians(),
                extract_f32(&kv, "RotationY").unwrap_or(0.0).to_radians(),
                extract_f32(&kv, "RotationZ").unwrap_or(0.0).to_radians(),
            ),
        },
    })
}

fn kv_to_point_light(kv: HashMap<String, IRDictionaryEntry>) -> anyhow::Result<MapEntry> {
    Ok(MapEntry {
        meta: MapEntryMeta {
            id: MapEntryID::new(),
            components: extract_string_vec(&kv, "Components").unwrap_or(vec![]),
        },
        data: MapEntryData::PointLight {
            location: extract_vec3(&kv, "Location").unwrap_or(Vec3::ZERO),
            color: extract_vec3(&kv, "Color").unwrap_or(Vec3::ONE),
            intensity: extract_f32(&kv, "Intensity").unwrap_or(1.0),
            linear_falloff: extract_bool(&kv, "LinearFalloff").unwrap_or(false),
            range: extract_f32(&kv, "Range").unwrap_or(10.0),
            shadow: extract_bool(&kv, "Shadow").unwrap_or(false),
        },
    })
}

fn kv_to_sun_light(kv: HashMap<String, IRDictionaryEntry>) -> anyhow::Result<MapEntry> {
    Ok(MapEntry {
        meta: MapEntryMeta {
            id: MapEntryID::new(),
            components: extract_string_vec(&kv, "Components").unwrap_or(vec![]),
        },
        data: MapEntryData::SunLight {
            direction: extract_vec3(&kv, "Direction").unwrap_or(Vec3::new(0.0, -1.0, 0.0)),
            color: extract_vec3(&kv, "Color").unwrap_or(Vec3::ONE),
            intensity: extract_f32(&kv, "Intensity").unwrap_or(1.0),
            ambient: extract_f32(&kv, "Ambient").unwrap_or(0.1),
            shadow: extract_bool(&kv, "Shadow").unwrap_or(false),
        },
    })
}

fn kv_to_spot_light(kv: HashMap<String, IRDictionaryEntry>) -> anyhow::Result<MapEntry> {
    Ok(MapEntry {
        meta: MapEntryMeta {
            id: MapEntryID::new(),
            components: extract_string_vec(&kv, "Components").unwrap_or(vec![]),
        },
        data: MapEntryData::SpotLight {
            location: extract_vec3(&kv, "Location").unwrap_or(Vec3::ZERO),
            direction: extract_vec3(&kv, "Direction").unwrap_or(Vec3::new(0.0, -1.0, 0.0)),
            color: extract_vec3(&kv, "Color").unwrap_or(Vec3::ONE),
            intensity: extract_f32(&kv, "Intensity").unwrap_or(1.0),
            range: extract_f32(&kv, "Range").unwrap_or(10.0),
            inner_cone_angle: extract_f32(&kv, "InnerConeAngle")
                .unwrap_or(15.0)
                .to_radians(),
            outer_cone_angle: extract_f32(&kv, "OuterConeAngle")
                .unwrap_or(30.0)
                .to_radians(),
            linear_falloff: extract_bool(&kv, "LinearFalloff").unwrap_or(false),
            shadow: extract_bool(&kv, "Shadow").unwrap_or(false),
        },
    })
}

pub fn parse_entries(dict: IRDictionary) -> anyhow::Result<Vec<MapEntry>> {
    let mut entries = vec![];
    let dict = dict.entries.iter().next().unwrap().as_map().unwrap();

    if let Some(objects) = dict.get("Objects") {
        let objects = objects.as_array().unwrap();
        for entry in objects {
            entries.push(kv_to_object(entry.as_map().unwrap().clone())?);
        }
    }

    if let Some(lights) = dict.get("PointLights") {
        let lights = lights.as_array().unwrap();
        for light in lights {
            entries.push(kv_to_point_light(light.as_map().unwrap().clone())?);
        }
    }

    if let Some(sun_lights) = dict.get("SunLights") {
        let sun_lights = sun_lights.as_array().unwrap();
        for sun_light in sun_lights {
            entries.push(kv_to_sun_light(sun_light.as_map().unwrap().clone())?);
        }
    }

    if let Some(spot_lights) = dict.get("SpotLights") {
        let spot_lights = spot_lights.as_array().unwrap();
        for spot_light in spot_lights {
            entries.push(kv_to_spot_light(spot_light.as_map().unwrap().clone())?);
        }
    }

    Ok(entries)
}
