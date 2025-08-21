use crate::systems::rendering::{CustomPassEvent, RenderPassIDs};
use dawn_assets::hub::{AssetHub, AssetInfoState};
use dawn_assets::query::AssetQueryID;
use dawn_ecs::Tick;
use dawn_graphics::input::{InputEvent, KeyCode};
use dawn_graphics::passes::events::RenderPassEvent;
use dawn_graphics::renderable::{ObjectMaterial, ObjectMesh};
use evenio::component::Component;
use evenio::entity::EntityId;
use evenio::event::{GlobalEvent, Insert, Receiver, Remove, Sender, Spawn};
use evenio::fetch::{Fetcher, Single};
use evenio::query::Or;
use evenio::world::World;
use log::info;

#[derive(Component)]
struct Timer {
    ticks: usize,
}

#[derive(Component)]
struct FreeAllAssetsQuery(AssetQueryID);

#[derive(GlobalEvent)]
struct DropAllAssets;
#[derive(GlobalEvent)]
struct AllAssetsDropped;

fn log_assets_handler(
    r: Receiver<AllAssetsDropped>,
    mut hub: Single<&mut AssetHub>,
    mut sender: Sender<(Spawn, Insert<FreeAllAssetsQuery>)>,
) {
    let mut sorted = hub.asset_infos();
    sorted.sort_by(|a, b| a.id.as_str().cmp(&b.id.as_str()));
    for (i, info) in sorted.iter().enumerate() {
        let state = match &info.state {
            AssetInfoState::Empty => "Empty".to_string(),
            AssetInfoState::IR(ram) => format!("IR ({} ram)", ram),
            AssetInfoState::Loaded { usage, rc } => {
                format!(
                    "Loaded ({} refs, {} ram, {} vram)",
                    rc, usage.ram, usage.vram
                )
            }
        };
        info!(
            "[{}] [{:<8}] {:<30} | {}",
            i,
            info.header.asset_type.to_string(),
            info.id.as_str(),
            state
        );
    }

    let qid = hub.query_free_all().unwrap();
    let query = sender.spawn();
    sender.insert(query, FreeAllAssetsQuery(qid));
}

fn drop_all_assest_handler(
    _: Receiver<DropAllAssets>,
    f: Fetcher<(EntityId, Or<&ObjectMesh, &ObjectMaterial>)>,
    ids: Single<&RenderPassIDs>,
    mut sender: Sender<(
        RenderPassEvent<CustomPassEvent>,
        Remove<ObjectMesh>,
        Remove<ObjectMaterial>,
        Spawn,
        Insert<Timer>,
    )>,
) {
    // Ask renderer to drop all owned assets
    sender.send(RenderPassEvent::new(
        ids.geometry,
        CustomPassEvent::DropAllAssets,
    ));
    sender.send(RenderPassEvent::new(
        ids.aabb,
        CustomPassEvent::DropAllAssets,
    ));

    // Remove all assets from the ECS
    for entity in f.iter() {
        match entity.1 {
            Or::Left(_) => {
                sender.remove::<ObjectMesh>(entity.0);
            }
            Or::Right(_) => {
                sender.remove::<ObjectMaterial>(entity.0);
            }
            Or::Both(_, _) => {
                sender.remove::<ObjectMesh>(entity.0);
                sender.remove::<ObjectMaterial>(entity.0);
            }
        }
    }

    // Assuming that the rendering thread is not throttled, so a logic update
    // period is the same as the rendering period.
    // It takes some time to drop assets:
    //    - Maximum of 3 frames to pass the event to the renderer
    //    - Maximum of 3 frames to empty the triple buffer used
    //      for Renderables streaming
    // You can experiment with this value to see how it affects the delay
    // of asset reload.
    const TIMER_INTERVAL: usize = 10; // In Frames

    // Spawn a timer to remove the assets when they are all dropped
    let id = sender.spawn();
    sender.insert(
        id,
        Timer {
            ticks: TIMER_INTERVAL,
        },
    );
}

fn timer_handler(
    _: Receiver<Tick>,
    mut f: Fetcher<(EntityId, &mut Timer)>,
    mut sender: Sender<(Remove<Timer>, AllAssetsDropped)>,
) {
    for timer in f.iter_mut() {
        if timer.1.ticks == 0 {
            info!("All assets dropped, removing timer");
            sender.remove::<Timer>(timer.0);
            sender.send(AllAssetsDropped);
        } else {
            timer.1.ticks -= 1;
        }
    }
}

pub fn setup_asset_reload_system(world: &mut World) {
    world.add_handler(drop_all_assest_handler);
    world.add_handler(timer_handler);
    world.add_handler(log_assets_handler);

    world.add_handler(
        move |r: Receiver<InputEvent>, mut sender: Sender<DropAllAssets>| {
            if matches!(r.event, InputEvent::KeyRelease(KeyCode::Function(1))) {
                sender.send(DropAllAssets);
            }
        },
    );
}
