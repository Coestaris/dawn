use crate::ui::{UIToRendererMessage, UIWorldConnection};
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::renderer::RendererMonitorEvent;
use evenio::fetch::Single;
use evenio::prelude::Receiver;
use evenio::world::World;

fn world_monitoring_handler(
    r: Receiver<WorldLoopMonitorEvent>,
    mut ui: Single<&mut UIWorldConnection>,
) {
    let _ = ui
        .sender
        .send(UIToRendererMessage::WorldMonitor(r.event.clone()));
}

fn renderer_monitoring_handler(
    r: Receiver<RendererMonitorEvent>,
    mut ui: Single<&mut UIWorldConnection>,
) {
    let _ = ui
        .sender
        .send(UIToRendererMessage::RendererMonitor(r.event.clone()));
}

pub fn setup_ui_system(world: &mut World, connection: UIWorldConnection) {
    let id = world.spawn();
    world.insert(id, connection);

    world.add_handler(world_monitoring_handler);
    world.add_handler(renderer_monitoring_handler);
}
