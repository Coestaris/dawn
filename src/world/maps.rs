use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::{AssetID, TypedAsset};
use dawn_graphics::ecs::{
    ObjectAreaLight, ObjectMesh, ObjectPointLight, ObjectPosition, ObjectRotation, ObjectScale,
    ObjectSpotLight, ObjectSunLight,
};
use dawn_graphics::gl::mesh::Mesh;
use evenio::component::Component;
use evenio::entity::EntityId;
use evenio::event::{Despawn, Insert, Receiver, Remove, Sender, Spawn};
use evenio::fetch::{Fetcher, Single};
use log::{debug, info, warn};
use crate::world::asset_swap::DropAllAssetsEvent;
use crate::world::dictionaries::DictionaryEntry;
use crate::world::Rotating;

#[derive(Component)]
struct MapLink {
    map_name: String,
    map_uid: usize,
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

    #[inline(never)]
    fn propagate_map(&self, sender: &mut SuperSender) {
        if let Some(map) = &self.map {
            let map = map.cast().as_map().unwrap();
            for object in map.objects.iter() {
                let uid = object.uid;
                let position = object.location;
                let rotation = object.rotation;
                let scale = object.scale;

                let id = sender.spawn();
                sender.insert(
                    id,
                    MapLink {
                        map_name: self.name.clone(),
                        map_uid: uid,
                    },
                );

                info!(
                    "Spawning entity {} at position {:?}, rotation {:?}, scale {:?}",
                    uid, position, rotation, scale
                );
                sender.insert(id, ObjectPosition(position));
                sender.insert(id, ObjectRotation(rotation));
                sender.insert(id, ObjectScale(scale));

                for component in object.components.iter() {
                    match component.as_str() {
                        "Rotating" => {
                            sender.insert(id, Rotating);
                        }
                        _ => {
                            warn!("Unknown component: {}", component);
                        }
                    }
                }
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
    mut sender: Sender<(Despawn)>,
) {
    dispatcher.map = None;

    // Despawn all entities with MapLink
    for (entity, _) in fetcher.iter() {
        info!("Despawning entity {:?}", entity);
        sender.despawn(entity.clone());
    }
}

pub fn setup_maps_system(world: &mut evenio::world::World) {
    static CURRENT_MAP: &str = "map1";

    let id = world.spawn();
    world.insert(id, MapDispatcher::new(CURRENT_MAP));

    world.add_handler(asset_events_handler);
    world.add_handler(drop_all_assets_handler);
}
