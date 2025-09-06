use crossbeam_channel::{Receiver, Sender};
use evenio::component::Component;
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::renderer::RendererMonitorEvent;

pub enum UIToRendererMessage {
    WorldMonitor(WorldLoopMonitorEvent),
    RendererMonitor(RendererMonitorEvent),
}

pub enum UIToWorldMessage {}

pub struct UIRendererConnection {
    pub sender: Sender<UIToWorldMessage>,
    pub receiver: Receiver<UIToRendererMessage>,
}

#[derive(Component)]
pub struct UIWorldConnection {
    pub sender: Sender<UIToRendererMessage>,
    pub receiver: Receiver<UIToWorldMessage>,
}

pub fn ui_bridge() -> (UIRendererConnection, UIWorldConnection) {
    let (ui_to_world_sender, ui_to_world_receiver) = crossbeam_channel::unbounded();
    let (ui_to_renderer_sender, ui_to_renderer_receiver) = crossbeam_channel::unbounded();

    let ui_renderer_connection = UIRendererConnection {
        sender: ui_to_world_sender,
        receiver: ui_to_renderer_receiver,
    };

    let ui_world_connection = UIWorldConnection {
        sender: ui_to_renderer_sender,
        receiver: ui_to_world_receiver,
    };

    (ui_renderer_connection, ui_world_connection)
}
