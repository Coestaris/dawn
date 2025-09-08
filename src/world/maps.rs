use crate::world::asset::{DropAllAssetsEvent, CURRENT_MAP};
use crate::world::dictionaries::{DictionaryEntry, MapUID};
use crate::world::{move_light_handler, rotate_handler, MovingByArrowKeys, Rotating};
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::{AssetID, TypedAsset};
use dawn_graphics::ecs::{
    ObjectAreaLight, ObjectMesh, ObjectPointLight, ObjectPosition, ObjectRotation, ObjectScale,
    ObjectSpotLight, ObjectSunLight,
};
use dawn_graphics::gl::mesh::Mesh;
use evenio::component::Component;
use evenio::entity::EntityId;
use evenio::event::{Despawn, Insert, Receiver, Sender, Spawn};
use evenio::fetch::{Fetcher, Single};
use log::{info, warn};

#[derive(Component)]
struct MapLink {
    map_name: String,
    map_uid: MapUID,
}

type SuperSender<'a> = Sender<
    'a,
    (
        Spawn,
        Insert<MapLink>,
        Insert<ObjectRotation>,
        Insert<ObjectPosition>,
        Insert<ObjectScale>,
        Insert<ObjectMesh>,
        Insert<ObjectPointLight>,
        Insert<ObjectSpotLight>,
        Insert<ObjectSunLight>,
        Insert<ObjectAreaLight>,

        // User components
        Insert<Rotating>,
        Insert<MovingByArrowKeys>,
    ),
>;

#[derive(Component)]
pub struct MapDispatcher {
    pub name: String,
    pub map: Option<TypedAsset<DictionaryEntry>>,
}

impl MapDispatcher {
    pub fn new(map: &str) -> Self {
        Self {
            name: map.to_string(),
            map: None,
        }
    }

    fn derive_components(&self, components: &Vec<String>, id: EntityId, sender: &mut SuperSender) {
        for component in components.iter() {
            match component.as_str() {
                "Rotating" => {
                    sender.insert(id, Rotating);
                }
                "MovingByArrowKeys" => {
                    sender.insert(id, MovingByArrowKeys);
                }
                _ => {
                    warn!("Unknown component: {}", component);
                }
            }
        }
    }

    #[inline(never)]
    fn propagate_map(&self, sender: &mut SuperSender) {
        if let Some(map) = &self.map {
            let map = map.cast().as_map().unwrap();
            for object in map.objects.iter() {
                let id = sender.spawn();
                sender.insert(
                    id,
                    MapLink {
                        map_name: self.name.clone(),
                        map_uid: object.uid,
                    },
                );
                sender.insert(id, ObjectPosition(object.location));
                sender.insert(id, ObjectRotation(object.rotation));
                sender.insert(id, ObjectScale(object.scale));
                self.derive_components(&object.components, id, sender);
            }
            for point_light in map.point_lights.iter() {
                let id = sender.spawn();
                sender.insert(
                    id,
                    MapLink {
                        map_name: self.name.clone(),
                        map_uid: point_light.uid,
                    },
                );

                sender.insert(id, ObjectPosition(point_light.location));
                sender.insert(
                    id,
                    ObjectPointLight {
                        color: point_light.color,
                        intensity: point_light.intensity,
                        range: point_light.range,
                    },
                );
                self.derive_components(&point_light.components, id, sender);
            }
        }
    }

    fn attach_asset(
        &self,
        hub: &AssetHub,
        aid: &AssetID,
        sender: &mut SuperSender,
        link_fetcher: &mut Fetcher<(EntityId, &MapLink)>,
    ) {
        if let Some(map) = &self.map {
            let map = map.cast().as_map().unwrap();
            for object in map.objects.iter() {
                // Found the mesh asset we want to assign
                if object.mesh.as_str() == aid.as_str() {
                    let mesh = hub.get_typed::<Mesh>(aid.clone()).unwrap();

                    // Now find the entity with the matching MapLink
                    for (entity, link) in link_fetcher.iter() {
                        if link.map_name == self.name && link.map_uid == object.uid {
                            info!(
                                "Assigning mesh {} to entity {:?} (UID: {})",
                                aid.as_str(),
                                entity,
                                object.uid
                            );
                            sender.insert(entity.clone(), ObjectMesh(mesh.clone()));
                        }
                    }
                }
            }
        }
    }

    #[inline(never)]
    pub fn dispatch(
        &mut self,
        hub: &AssetHub,
        event: &AssetHubEvent,
        sender: &mut SuperSender,
        link_fetcher: &mut Fetcher<(EntityId, &MapLink)>,
    ) {
        match event {
            AssetHubEvent::AssetLoaded(aid) if aid.as_str() == self.name => {
                let map = hub.get_typed::<DictionaryEntry>(aid.clone()).unwrap();
                info!("Loaded map: {}", aid.as_str());
                self.map = Some(map);
                self.propagate_map(sender);
            }
            AssetHubEvent::AssetLoaded(aid) => {
                self.attach_asset(hub, aid, sender, link_fetcher);
            }

            _ => {}
        }
    }
}

fn asset_events_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&AssetHub>,
    mut dispatcher: Single<&mut MapDispatcher>,
    mut link_fetcher: Fetcher<(EntityId, &MapLink)>,
    mut sender: SuperSender,
) {
    dispatcher.dispatch(hub.0, r.event, &mut sender, &mut link_fetcher);
}

fn drop_all_assets_handler(
    _: Receiver<DropAllAssetsEvent>,
    mut dispatcher: Single<&mut MapDispatcher>,
    fetcher: Fetcher<(EntityId, &MapLink)>,
    mut sender: Sender<Despawn>,
) {
    dispatcher.map = None;

    // Despawn all entities with MapLink
    for (entity, _) in fetcher.iter() {
        info!("Despawning entity {:?}", entity);
        sender.despawn(entity.clone());
    }
}

pub fn setup_maps_system(world: &mut evenio::world::World) {
    let id = world.spawn();
    world.insert(id, MapDispatcher::new(CURRENT_MAP));

    world.add_handler(asset_events_handler);
    world.add_handler(drop_all_assets_handler);

    world.add_handler(rotate_handler);
    world.add_handler(move_light_handler);
}
