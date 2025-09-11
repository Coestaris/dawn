use crate::world::devtools::WorldStatistics;
use crossbeam_channel::{Receiver, Sender};
use dawn_assets::hub::AssetInfo;
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::renderer::RendererMonitorEvent;
use evenio::component::Component;

pub enum DevtoolsToRendererMessage {
    WorldMonitor(WorldLoopMonitorEvent, WorldStatistics),
    RendererMonitor(RendererMonitorEvent),
    AssetsEnumerated(Vec<AssetInfo>),
}

pub enum DevtoolsToWorldMessage {
    EnumerateAssets,
}

pub struct DevtoolsRendererConnection {
    pub sender: Sender<DevtoolsToWorldMessage>,
    pub receiver: Receiver<DevtoolsToRendererMessage>,
}

#[derive(Component)]
pub struct DevtoolsWorldConnection {
    pub sender: Sender<DevtoolsToRendererMessage>,
    pub receiver: Receiver<DevtoolsToWorldMessage>,
}

pub fn devtools_bridge() -> (DevtoolsRendererConnection, DevtoolsWorldConnection) {
    let (ui_to_world_sender, ui_to_world_receiver) = crossbeam_channel::unbounded();
    let (ui_to_renderer_sender, ui_to_renderer_receiver) = crossbeam_channel::unbounded();

    let ui_renderer_connection = DevtoolsRendererConnection {
        sender: ui_to_world_sender,
        receiver: ui_to_renderer_receiver,
    };

    let ui_world_connection = DevtoolsWorldConnection {
        sender: ui_to_renderer_sender,
        receiver: ui_to_world_receiver,
    };

    (ui_renderer_connection, ui_world_connection)
}
