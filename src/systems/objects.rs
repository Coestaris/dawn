use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_graphics::gl::entities::material::Material;
use dawn_graphics::gl::entities::mesh::Mesh;
use dawn_graphics::renderable::{
    ObjectMaterial, ObjectMesh, ObjectPosition, ObjectRotation, ObjectScale,
};
use evenio::component::Component;
use evenio::entity::EntityId;
use evenio::event::{Insert, Receiver, Sender};
use evenio::fetch::{Fetcher, Single};
use glam::{Quat, Vec3};
use dawn_ecs::events::TickEvent;

#[derive(Component)]
pub struct GameObject;

fn rotate_handler(t: Receiver<TickEvent>, rotation: Fetcher<&mut ObjectRotation>) {
    for f in rotation {
        f.0 =
            f.0 * Quat::from_rotation_y(t.event.delta * 0.3) * Quat::from_rotation_x(t.event.delta * 0.1);
    }
}

fn map_assets_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    f: Fetcher<(EntityId, &GameObject)>,
    mut insert: Sender<(Insert<ObjectMesh>, Insert<ObjectMaterial>)>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) if *id == "barrel".into() => {
            let mesh = hub.get_typed::<Mesh>("barrel".into()).unwrap();
            for (id, _) in f {
                insert.insert(id, ObjectMesh(mesh.clone()));
            }
        }
        AssetHubEvent::AssetLoaded(id) if *id == "barrel_material".into() => {
            let material = hub.get_typed::<Material>("barrel_material".into()).unwrap();
            for (id, _) in f {
                insert.insert(id, ObjectMaterial(material.clone()));
            }
        }

        _ => {}
    }
}

pub fn setup_objects_system(world: &mut evenio::world::World) {
    for _ in 0..3 {
        let id = world.spawn();
        world.insert(id, GameObject);
        world.insert(id, ObjectRotation(Quat::IDENTITY));
        world.insert(id, ObjectScale(Vec3::splat(4.0)));
        world.insert(id, ObjectPosition(Vec3::new(0.0, 0.0, -10.0)));
    }

    world.add_handler(rotate_handler);
    world.add_handler(map_assets_handler);
}
