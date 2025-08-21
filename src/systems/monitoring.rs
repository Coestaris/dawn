use dawn_ecs::MainLoopMonitoring;
use dawn_graphics::renderer::RendererMonitoring;
use evenio::event::Receiver;
use evenio::world::World;
use log::info;

fn main_loop_monitoring_handler(r: Receiver<MainLoopMonitoring>) {
    info!(
        "Main loop: {:.1}tps ({:.1}%)",
        r.event.tps.average(),
        r.event.load.average() * 100.0
    );
}

fn renderer_monitoring_handler(r: Receiver<RendererMonitoring>) {
    info!(
        "Renderer: {:.1} FPS. {:.1}/{:.1}",
        r.event.fps.average(),
        r.event.render.average().as_millis(),
        r.event.view.average().as_millis(),
    );
}

pub fn setup_monitoring_system(world: &mut World) {
    world.add_handler(main_loop_monitoring_handler);
    world.add_handler(renderer_monitoring_handler);
}
