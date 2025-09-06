use crate::ui::{UIToRendererMessage, UIToWorldMessage, UIWorldConnection};
use dawn_assets::hub::AssetHub;
use dawn_ecs::events::TickEvent;
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::ecs::{
    ObjectAreaLight, ObjectMesh, ObjectPointLight, ObjectSpotLight, ObjectSunLight,
};
use dawn_graphics::renderer::RendererMonitorEvent;
use evenio::entity::EntityId;
use evenio::fetch::{Fetcher, Single};
use evenio::prelude::Receiver;
use evenio::world::World;

pub struct WorldStatistics {
    pub entities: usize,
    pub drawables: usize,
    pub point_lights: usize,
    pub spot_lights: usize,
    pub sun_lights: usize,
    pub area_lights: usize,
}

fn world_monitoring_handler(
    r: Receiver<WorldLoopMonitorEvent>,
    meshes: Fetcher<&ObjectMesh>,
    entities: Fetcher<EntityId>,
    point_lights: Fetcher<&ObjectPointLight>,
    spot_lights: Fetcher<&ObjectSpotLight>,
    sun_lights: Fetcher<&ObjectSunLight>,
    area_lights: Fetcher<&ObjectAreaLight>,
    mut ui: Single<&mut UIWorldConnection>,
) {
    let stats = WorldStatistics {
        entities: entities.iter().count(),
        drawables: meshes.iter().count(),
        point_lights: point_lights.iter().count(),
        spot_lights: spot_lights.iter().count(),
        sun_lights: sun_lights.iter().count(),
        area_lights: area_lights.iter().count(),
    };
    let _ = ui
        .sender
        .send(UIToRendererMessage::WorldMonitor(r.event.clone(), stats));
}

fn renderer_monitoring_handler(
    r: Receiver<RendererMonitorEvent>,
    mut ui: Single<&mut UIWorldConnection>,
) {
    let _ = ui
        .sender
        .send(UIToRendererMessage::RendererMonitor(r.event.clone()));
}

fn recv_messages_from_renderer_handler(
    _: Receiver<TickEvent>,
    hub: Single<&mut AssetHub>,
    mut ui: Single<&mut UIWorldConnection>,
) {
    while let Ok(msg) = ui.receiver.try_recv() {
        match msg {
            UIToWorldMessage::EnumerateAssets => {
                let mut infos = hub.asset_infos();
                // Sort by type and then by name
                infos.sort_by(|a, b| a.header.id.cmp(&b.header.id));
                let _ = ui.sender.send(UIToRendererMessage::AssetsEnumerated(infos));
            }
        }
    }
}

pub fn setup_ui_system(world: &mut World, connection: UIWorldConnection) {
    let id = world.spawn();
    world.insert(id, connection);

    world.add_handler(world_monitoring_handler);
    world.add_handler(renderer_monitoring_handler);
    world.add_handler(recv_messages_from_renderer_handler);
}
