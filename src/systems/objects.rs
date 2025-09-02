use crate::components::input::InputHolder;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_ecs::events::TickEvent;
use dawn_graphics::ecs::{
    ObjectMaterial, ObjectMesh, ObjectPointLight, ObjectPosition, ObjectRotation, ObjectScale,
};
use dawn_graphics::gl::mesh::Mesh;
use dawn_graphics::input::KeyCode;
use evenio::component::Component;
use evenio::entity::EntityId;
use evenio::event::{Insert, Receiver, Sender};
use evenio::fetch::{Fetcher, Single};
use glam::{Quat, Vec3};

#[derive(Component)]
pub struct GameObject;
#[derive(Component)]
pub struct Map;

fn rotate_handler(t: Receiver<TickEvent>, f: Fetcher<(&mut ObjectRotation, &GameObject)>) {
    for (rot, _) in f {
        rot.0 = rot.0
            * Quat::from_rotation_y(t.event.delta * 0.3)
            * Quat::from_rotation_x(t.event.delta * 0.1);
    }
}

fn move_light_handler(
    t: Receiver<TickEvent>,
    holder: Single<&mut InputHolder>,
    f: Fetcher<(&mut ObjectPosition, &ObjectPointLight)>,
) {
    for (pos, _) in f {
        const SPEED: f32 = 10.0;
        if holder.key_pressed(KeyCode::Up) {
            pos.0.y += t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::Down) {
            pos.0.y -= t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::Left) {
            pos.0.x -= t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::Right) {
            pos.0.x += t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::PageUp) {
            pos.0.z += t.event.delta * SPEED;
        }
        if holder.key_pressed(KeyCode::PageDown) {
            pos.0.z -= t.event.delta * SPEED;
        }
    }
}

fn map_assets_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    f1: Fetcher<(EntityId, &GameObject)>,
    f2: Fetcher<(EntityId, &Map)>,
    mut insert: Sender<(Insert<ObjectMesh>, Insert<ObjectMaterial>)>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) if *id == "barrel".into() => {
            let mesh = hub.get_typed::<Mesh>("barrel".into()).unwrap();
            for (id, _) in f1 {
                insert.insert(id, ObjectMesh(mesh.clone()));
            }
        }
        AssetHubEvent::AssetLoaded(id) if *id == "sponza".into() => {
            let mesh = hub.get_typed::<Mesh>("sponza".into()).unwrap();
            for (id, _) in f2 {
                insert.insert(id, ObjectMesh(mesh.clone()));
            }
        }
        _ => {}
    }
}

pub fn setup_objects_system(world: &mut evenio::world::World) {
    for i in 0..3 {
        for j in 0..3 {
            let id = world.spawn();
            world.insert(id, GameObject);
            world.insert(id, ObjectRotation(Quat::IDENTITY));
            world.insert(id, ObjectScale(Vec3::splat(2.0)));
            world.insert(
                id,
                ObjectPosition(Vec3::new(i as f32 * 15.0, 5.0, 0.0 + j as f32 * 15.0)),
            );
        }
    }

    let id = world.spawn();
    world.insert(id, Map);
    world.insert(id, ObjectRotation(Quat::IDENTITY));
    world.insert(id, ObjectScale(Vec3::splat(4.0)));
    world.insert(id, ObjectPosition(Vec3::new(0.0, 0.0, 0.0)));

    let id = world.spawn();
    world.insert(
        id,
        ObjectPointLight {
            color: Vec3::new(1.0, 0.8, 0.6),
            intensity: 50.0,
        },
    );
    world.insert(id, ObjectPosition(Vec3::new(0.0, 10.0, 0.0)));

    world.add_handler(move_light_handler);
    world.add_handler(rotate_handler);
    world.add_handler(map_assets_handler);
}
