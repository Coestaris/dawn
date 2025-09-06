use crate::ui::{UIRendererConnection, UIToRendererMessage};
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::renderer::RendererMonitorEvent;
use imgui::Ui;
use std::cell::RefCell;
use std::rc::Rc;

pub struct RenderingConfigInner {
    wireframe: bool,
}

pub struct RenderingConfig(Rc<RefCell<RenderingConfigInner>>);

impl Clone for RenderingConfig {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl RenderingConfig {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(RenderingConfigInner {
            wireframe: false,
        })))
    }

    pub fn borrow(&self) -> std::cell::Ref<RenderingConfigInner> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> std::cell::RefMut<RenderingConfigInner> {
        self.0.borrow_mut()
    }
}

pub struct UI {
    connection: UIRendererConnection,
    config: RenderingConfig,

    renderer_monitor_event: Option<RendererMonitorEvent>,
    world_monitor_event: Option<WorldLoopMonitorEvent>,

    run: bool,
}

impl UI {
    pub fn new(config: RenderingConfig, connection: UIRendererConnection) -> Self {
        Self {
            connection,
            config,
            renderer_monitor_event: None,
            world_monitor_event: None,
            run: true,
        }
    }

    pub fn render(&mut self, ui: &mut Ui) {
        if let Ok(message) = self.connection.receiver.try_recv() {
            match message {
                UIToRendererMessage::RendererMonitor(event) => {
                    self.renderer_monitor_event = Some(event);
                }
                UIToRendererMessage::WorldMonitor(event) => {
                    self.world_monitor_event = Some(event);
                }
            }
        }

        // Show FPS
        if let ((Some(renderer_event), Some(world_event))) =
            (&self.renderer_monitor_event, &self.world_monitor_event)
        {
            ui.window("Renderer Monitor")
                .size([300.0, 200.0], imgui::Condition::FirstUseEver)
                .position([10.0, 10.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    const WORLD_COLOR: [f32; 4] = [1.0, 0.7, 0.1, 1.0];
                    const RENDERING_COLOR: [f32; 4] = [0.1, 0.7, 1.0, 1.0];
                    ui.text_colored(
                        WORLD_COLOR,
                        format!(
                            "TPS: {:.1}/{:.1}/{:.1}",
                            world_event.tps.min(),
                            world_event.tps.average(),
                            world_event.tps.max(),
                        ),
                    );
                    ui.text_colored(
                        WORLD_COLOR,
                        format!(
                            "Load: {:.1}/{:.1}/{:.1}%",
                            world_event.load.min() * 100.0,
                            world_event.load.average() * 100.0,
                            world_event.load.max() * 100.0
                        ),
                    );
                    ui.text_colored(
                        WORLD_COLOR,
                        format!(
                            "Tick time: {:.1}/{:.1}/{:.1} ms",
                            world_event.cycle_time.min().as_millis(),
                            world_event.cycle_time.average().as_millis(),
                            world_event.cycle_time.max().as_millis()
                        ),
                    );

                    ui.separator();

                    ui.text_colored(
                        RENDERING_COLOR,
                        format!(
                            "FPS: {:.1}/{:.1}/{:.1}",
                            renderer_event.fps.min(),
                            renderer_event.fps.average(),
                            renderer_event.fps.max(),
                        ),
                    );
                    ui.text_colored(
                        RENDERING_COLOR,
                        format!(
                            "Load: {:.1}/{:.1}/{:.1}%",
                            renderer_event.load.min() * 100.0,
                            renderer_event.load.average() * 100.0,
                            renderer_event.load.max() * 100.0
                        ),
                    );

                    ui.text_colored(
                        RENDERING_COLOR,
                        format!(
                            "Primitives: {:.1e}/{:.1e}/{:.1e}. ",
                            renderer_event.drawn_primitives.min(),
                            renderer_event.drawn_primitives.average(),
                            renderer_event.drawn_primitives.max(),
                        ),
                    );
                    ui.text_colored(
                        RENDERING_COLOR,
                        format!(
                            "Draw Calls: {:.1e}/{:.1e}/{:.1e}. ",
                            renderer_event.draw_calls.min(),
                            renderer_event.draw_calls.average(),
                            renderer_event.draw_calls.max(),
                        ),
                    );

                    ui.text_colored(
                        RENDERING_COLOR,
                        format!(
                            "Render: {:.1}/{:.1}/{:.1} ms",
                            renderer_event.render.min().as_millis(),
                            renderer_event.render.average().as_millis(),
                            renderer_event.render.max().as_millis(),
                        ),
                    );
                    ui.text_colored(
                        RENDERING_COLOR,
                        format!(
                            "View: {:.1}/{:.1}/{:.1} ms",
                            renderer_event.view.min().as_millis(),
                            renderer_event.view.average().as_millis(),
                            renderer_event.view.max().as_millis()
                        ),
                    );

                    ui.text_colored(
                        RENDERING_COLOR,
                        format!(
                            "Events: {:.1}/{:.1}/{:.1} ms",
                            renderer_event.events.min().as_millis(),
                            renderer_event.events.average().as_millis(),
                            renderer_event.events.max().as_millis()
                        ),
                    );

                    ui.separator();

                    ui.text_colored(RENDERING_COLOR, "Pass Times:");
                    for (pass_name, pass_time) in &renderer_event.passes {
                        ui.text_colored(
                            RENDERING_COLOR,
                            format!(
                                "{}: {:.1}/{:.1}/{:.1} ms",
                                pass_name,
                                pass_time.min().as_millis(),
                                pass_time.average().as_millis(),
                                pass_time.max().as_millis()
                            ),
                        );
                    }
                });
        }
    }
}
