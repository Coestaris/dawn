use crate::systems::rendering::{CustomPassEvent, RenderPassIDs};
use dawn_assets::hub::{AssetHub, AssetHubEvent, AssetInfoState};
use dawn_assets::requests::{AssetRequest, AssetRequestID, AssetRequestQuery};
use dawn_assets::AssetType;
use dawn_ecs::events::{ExitEvent, TickEvent};
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
    and_then: AndThen,
    ticks: usize,
}

#[derive(Component)]
struct FreeAllAssetsRequest(AssetRequestID, pub AndThen);

#[derive(GlobalEvent)]
pub struct DropAllAssetsEvent(pub AndThen);

#[derive(Clone, Debug)]
pub enum AndThen {
    StopMainLoop,
    ReloadAssets,
}

pub fn load_assets(hub: &mut AssetHub) {
    hub.request(AssetRequest::Enumerate);
    hub.request(AssetRequest::Load(AssetRequestQuery::All));
}

pub fn free_assets(hub: &mut AssetHub) -> AssetRequestID {
    hub.request(AssetRequest::FreeNoDeps(AssetRequestQuery::ByType(
        AssetType::Mesh,
    )));
    // Materials are holding the textures. Free them first.
    // Hoping that no one else is using them otherwise it's circular reference.
    //   Textures are stored in Material so cannot be freed first
    //   Materials depends on Textures so cannot be freed first
    hub.request(AssetRequest::FreeNoDeps(AssetRequestQuery::ByType(
        AssetType::Material,
    )));
    // Same with Fonts
    hub.request(AssetRequest::FreeNoDeps(AssetRequestQuery::ByType(
        AssetType::Font,
    )));
    hub.request(AssetRequest::Free(AssetRequestQuery::All))
}

#[derive(GlobalEvent)]
struct AllAssetsDroppedEvent(pub AndThen);

fn drop_all_assets_handler(
    r: Receiver<DropAllAssetsEvent>,
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
    info!("DropAllAssetsEvent received: {:?}", r.event.0);

    // Ask renderer to drop all owned assets
    let broadcast = [ids.geometry, ids.aabb, ids.ui];
    for id in broadcast.iter() {
        sender.send(RenderPassEvent::new(*id, CustomPassEvent::DropAllAssets));
    }

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
    const TIMER_INTERVAL: usize = 5; // In Frames

    // Spawn a timer to remove the assets when they are all dropped
    let id = sender.spawn();
    sender.insert(
        id,
        Timer {
            and_then: r.event.0.clone(),
            ticks: TIMER_INTERVAL,
        },
    );
}

fn timer_handler(
    _: Receiver<TickEvent>,
    mut f: Fetcher<(EntityId, &mut Timer)>,
    mut sender: Sender<(Remove<Timer>, AllAssetsDroppedEvent)>,
) {
    for timer in f.iter_mut() {
        if timer.1.ticks == 0 {
            info!("All assets dropped, removing timer");
            sender.remove::<Timer>(timer.0);
            sender.send(AllAssetsDroppedEvent(timer.1.and_then.clone()));
        } else {
            timer.1.ticks -= 1;
        }
    }
}

fn print_assets(hub: &AssetHub) {
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
            "[{:<3}] [{:<8}] {:<45} | {}",
            i,
            info.header.asset_type.to_string(),
            info.id.as_str(),
            state
        );
    }
}

fn free_assets_handler(
    r: Receiver<AllAssetsDroppedEvent>,
    mut hub: Single<&mut AssetHub>,
    mut sender: Sender<(Spawn, Insert<FreeAllAssetsRequest>)>,
) {
    print_assets(*hub);
    let request = sender.spawn();
    sender.insert(
        request,
        FreeAllAssetsRequest(free_assets(*hub), r.event.0.clone()),
    );
}

fn request_finished(
    r: Receiver<AssetHubEvent>,
    f: Fetcher<(EntityId, &FreeAllAssetsRequest)>,
    mut hub: Single<&mut AssetHub>,
    mut sender: Sender<(ExitEvent, Remove<FreeAllAssetsRequest>)>,
) {
    let (rid, and_then) = match f.iter().next() {
        Some((_, req)) => (req.0, req.1.clone()),
        None => return,
    };
    if let AssetHubEvent::RequestFinished(id, Ok(())) = r.event {
        if *id == rid {
            match and_then {
                AndThen::ReloadAssets => {
                    info!("Free all assets request finished, reloading assets");
                    load_assets(*hub);
                }
                AndThen::StopMainLoop => {
                    // The request to free all assets is finished
                    // The actual removal of assets from ECS is done in the timer handler
                    info!("Free all assets request finished");
                    sender.send(ExitEvent);
                }
            }
            sender.remove::<FreeAllAssetsRequest>(f.iter().next().unwrap().0);
        }
    }
}

pub fn setup_asset_swap_system(world: &mut World) {
    // First we wait for DropAllAssets event
    world.add_handler(drop_all_assets_handler);
    // Then we wait for the timer to finish
    world.add_handler(timer_handler);
    // Then we request the AssetHub to free all assets
    world.add_handler(free_assets_handler);
    // After the AssetHub finished the request, we stop the main loop
    world.add_handler(request_finished);
}
