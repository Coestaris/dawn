use crate::assets::blob::BlobAssetFactory;
use crate::assets::dict::DictionaryAssetFactory;
use crate::assets::reader::{Reader, ReaderBackend};
use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::RenderingEvent;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::requests::{AssetRequest, AssetRequestID, AssetRequestQuery};
use dawn_assets::AssetType;
use dawn_ecs::events::{ExitEvent, TickEvent};
use dawn_graphics::ecs::{InvalidateRendererCache, ObjectMesh};
use dawn_graphics::passes::events::RenderPassEvent;
use evenio::component::Component;
use evenio::entity::EntityId;
use evenio::event::{GlobalEvent, Insert, Receiver, Remove, Sender, Spawn};
use evenio::fetch::{Fetcher, Single};
use evenio::prelude::World;
use log::info;
use std::sync::Arc;

pub const CURRENT_MAP: &str = "map1";
pub const CURRENT_SKYBOX: &str = "skybox1";

pub const APPLICATION_ICON_BLOB_ID: &str = "icon_blob";
pub const SUN_LIGHT_TEXTURE: &str = "sun_light";
pub const POINT_LIGHT_TEXTURE: &str = "point_light";

fn assets_failed_handler(r: Receiver<AssetHubEvent>) {
    match r.event {
        AssetHubEvent::RequestFinished(request, Err(message)) => {
            panic!("Asset Request Failed {:?}: {:?}", request, message);
        }
        _ => {}
    }
}

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
    StopWorldLoop,
    ReloadAssets,
}

#[derive(GlobalEvent)]
struct AllAssetsDroppedEvent(pub AndThen);

fn drop_all_assets_in_renderer_handler(
    _: Receiver<DropAllAssetsEvent>,
    mut sender: Sender<InvalidateRendererCache>,
) {
    sender.send(InvalidateRendererCache);
}

fn drop_all_assets_in_pipeline_handler(
    _: Receiver<DropAllAssetsEvent>,
    dispatcher: Single<&RenderDispatcher>,
    sender: Sender<RenderPassEvent<RenderingEvent>>,
) {
    dispatcher.dispatch_drop_assets(sender);
}

fn drop_all_assets_in_world_handler(
    r: Receiver<DropAllAssetsEvent>,
    f: Fetcher<(EntityId, &ObjectMesh)>,
    mut sender: Sender<(Remove<ObjectMesh>, Spawn, Insert<Timer>)>,
) {
    info!("DropAllAssetsEvent received: {:?}", r.event.0);

    // Remove all assets from the ECS
    for (entity, _) in f.iter() {
        sender.remove::<ObjectMesh>(entity);
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

fn free_assets_handler(
    r: Receiver<AllAssetsDroppedEvent>,
    mut hub: Single<&mut AssetHub>,
    mut sender: Sender<(Spawn, Insert<FreeAllAssetsRequest>)>,
) {
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
                AndThen::StopWorldLoop => {
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

fn load_assets(hub: &mut AssetHub) {
    hub.request(AssetRequest::Enumerate);
    hub.request(AssetRequest::Load(AssetRequestQuery::ByType(
        AssetType::Blob,
    )));
    hub.request(AssetRequest::Load(AssetRequestQuery::ByType(
        AssetType::Shader,
    )));
    hub.request(AssetRequest::LoadNoDeps(AssetRequestQuery::ByType(
        AssetType::Dictionary,
    )));
    hub.request(AssetRequest::Load(AssetRequestQuery::All));
}

fn free_assets(hub: &mut AssetHub) -> AssetRequestID {
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

pub fn setup_assets_system(
    world: &mut World,
    reader_backend: Arc<dyn ReaderBackend>,
    mut hub: AssetHub,
) {
    // Request initial assets
    load_assets(&mut hub);

    // Setup the asset reader thread
    // It will read the DAC file and load assets into the AssetHub
    let reader = Reader::new(reader_backend, hub.get_read_binding());
    reader.attach_to_ecs(world);

    // Setup the dictionary factory. It's quite unique because it the only
    // one that defined on our side.
    let mut dictionary_factory = DictionaryAssetFactory::new();
    dictionary_factory.bind(hub.get_factory_biding(AssetType::Dictionary));
    dictionary_factory.attach_to_ecs(world);
    let mut blob_factory = BlobAssetFactory::new();
    blob_factory.bind(hub.get_factory_biding(AssetType::Blob));
    blob_factory.attach_to_ecs(world);

    // Move the AssetHub into the ECS
    hub.attach_to_ecs(world);
    world.add_handler(assets_failed_handler);
    // Former 'asset swap' system
    // First we wait for DropAllAssets event
    world.add_handler(drop_all_assets_in_renderer_handler);
    world.add_handler(drop_all_assets_in_world_handler);
    world.add_handler(drop_all_assets_in_pipeline_handler);
    // Then we wait for the timer to finish
    world.add_handler(timer_handler);
    // Then we request the AssetHub to free all assets
    world.add_handler(free_assets_handler);
    // After the AssetHub finished the request, we stop the main loop

    world.add_handler(request_finished);
}
