use crate::components::imui::{Style, UICommand};
use crate::systems::asset_swap::DropAllAssetsEvent;
use build_info::semver::Op;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::TypedAsset;
use dawn_ecs::events::InterSyncEvent;
use dawn_ecs::main_loop::MainLoopMonitorEvent;
use dawn_graphics::gl::font::Font;
use dawn_graphics::input::{InputEvent, KeyCode};
use dawn_graphics::renderer::RendererMonitorEvent;
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::Single;
use evenio::world::World;
use glam::{Vec2, Vec4};
use log::{debug, info};
use triple_buffer::{triple_buffer, Input, Output};

#[derive(Component)]
struct UISystem {
    input: Input<Vec<UICommand>>,
    font: Option<TypedAsset<Font>>,
    main_loop: Option<MainLoopMonitorEvent>,
    renderer: Option<RendererMonitorEvent>,
    viewport: Option<Vec2>,
    detailed: bool,
}

fn toggle_detailed_handler(r: Receiver<InputEvent>, mut ui: Single<&mut UISystem>) {
    match r.event {
        InputEvent::KeyPress(KeyCode::Function(1)) => {
            ui.detailed = !ui.detailed;
            debug!("Toggled detailed UI: {}", ui.detailed);
        }
        InputEvent::Resize { width, height } => {
            ui.viewport = Some(Vec2::new(*width as f32, *height as f32));
        }
        _ => {}
    }
}

fn drop_all_assets_handler(r: Receiver<DropAllAssetsEvent>, mut ui: Single<&mut UISystem>) {
    debug!("Dropping all UI assets");
    ui.font = None;

    // Flush the content of the input buffer
    // The triple buffer must be cleared... you guessed it... three times
    // It's ugly, but it works
    let vec = ui.input.input_buffer_mut();
    vec.clear();
    ui.input.publish();
    let vec = ui.input.input_buffer_mut();
    vec.clear();
    ui.input.publish();
    let vec = ui.input.input_buffer_mut();
    vec.clear();
    ui.input.publish();
}

fn main_loop_monitoring_handler(r: Receiver<MainLoopMonitorEvent>, mut ui: Single<&mut UISystem>) {
    ui.main_loop = Some(r.event.clone());
}

fn renderer_monitoring_handler(r: Receiver<RendererMonitorEvent>, mut ui: Single<&mut UISystem>) {
    ui.renderer = Some(r.event.clone());
}

struct Stacked<'a> {
    position: Vec2,
    viewport: Vec2,
    output: &'a mut Vec<UICommand>,
    style: &'a Style,
}

impl<'a> Stacked<'a> {
    pub fn new(
        position: Vec2,
        viewport: Vec2,
        output: &'a mut Vec<UICommand>,
        style: &'a Style,
    ) -> Self {
        Self {
            position,
            viewport,
            output,
            style,
        }
    }

    pub fn color(&mut self, color: Vec4) {
        self.output.push(UICommand::ChangeColor(color));
    }

    pub fn push_down(&mut self, str: String) {
        let font = self.style.font.cast();
        let dim = font.text_dimensions(str.as_str());
        self.output.push(UICommand::Text(self.position, str));
        self.position.y += dim.y * self.style.scale;
    }

    pub fn push_up(&mut self, str: String) {
        let font = self.style.font.cast();
        let dim = font.text_dimensions(str.as_str());
        self.position.y -= dim.y * self.style.scale;
        self.output.push(UICommand::Text(self.position, str));
    }

    pub fn push_down_str(&mut self, str: &'static str) {
        let font = self.style.font.cast();
        let dim = font.text_dimensions(str);
        self.output.push(UICommand::StaticText(self.position, str));
        self.position.y += dim.y * self.style.scale;
    }

    pub fn push_up_str(&mut self, str: &'static str) {
        let font = self.style.font.cast();
        let dim = font.text_dimensions(str);
        self.position.y -= dim.y * self.style.scale;
        self.output.push(UICommand::StaticText(self.position, str));
    }
}

fn stream_ui_handler(_: Receiver<InterSyncEvent>, mut ui: Single<&mut UISystem>) {
    let font = ui.font.as_ref().map(|f| f.clone());
    let main_loop = ui.main_loop.as_ref().map(|f| f.clone());
    let renderer = ui.renderer.as_ref().map(|f| f.clone());
    let viewport = ui.viewport.unwrap_or(Vec2::new(800.0, 600.0));
    let detailed = ui.detailed;

    let vec = ui.input.input_buffer_mut();
    vec.clear();

    if let Some(font) = font {
        let style = Style {
            font: font.clone(),
            scale: 0.5,
        };
        vec.push(UICommand::ApplyStyle(style.clone()));

        if !detailed {
            {
                let stacked = &mut Stacked::new(Vec2::new(5.0, viewport.y), viewport, vec, &style);
                // Change color to yellow
                stacked.color(Vec4::new(1.0, 1.0, 0.0, 1.0));
                stacked.push_up_str("Press F1 for detailed info");
            }

            let stacked = &mut Stacked::new(Vec2::new(5.0, 5.0), viewport, vec, &style);
            stacked.color(Vec4::new(0.0, 1.0, 0.0, 1.0));
            if let Some(main_loop) = main_loop {
                stacked.push_down(format!(
                    "TPS: {:.1} ({:.1}%)",
                    main_loop.tps.average(),
                    main_loop.load.average() * 100.0
                ));
            }
            // Light blue color
            stacked.color(Vec4::new(0.1, 0.7, 1.0, 1.0));
            if let Some(renderer) = renderer {
                stacked.push_down(format!(
                    "FPS: {:.1} ({:.1}%)",
                    renderer.fps.average(),
                    renderer.load.average() * 100.0
                ));
            }
        } else {
            {
                let stacked = &mut Stacked::new(Vec2::new(5.0, viewport.y), viewport, vec, &style);
                // Change color to yellow
                stacked.color(Vec4::new(1.0, 1.0, 0.0, 1.0));
                stacked.push_up_str("WASD/Shift/Space + Mouse Drag - Move camera");
                stacked.push_up_str("F11  - Toggle fullscreen");
                stacked.push_up_str("F5   - Refresh assets");
                stacked.push_up_str("F4   - Toggle AABB");
                stacked.push_up_str("F3   - Toggle wireframe");
                stacked.push_up_str("F1   - Toggle detailed info");
                stacked.push_up_str("ESC  - Quit");
                stacked.push_up_str("Controls: ");
            }

            let stacked = &mut Stacked::new(Vec2::new(5.0, 5.0), viewport, vec, &style);
            stacked.color(Vec4::new(0.0, 1.0, 0.0, 1.0));
            if let Some(main_loop) = main_loop {
                stacked.push_down(format!(
                    "TPS: {:.1}/{:.1}/{:.1}. Load: {:.1}/{:.1}/{:.1}%",
                    main_loop.tps.min(),
                    main_loop.tps.average(),
                    main_loop.tps.max(),
                    main_loop.load.min() * 100.0,
                    main_loop.load.average() * 100.0,
                    main_loop.load.max() * 100.0
                ));
                stacked.push_down(format!(
                    "Tick time: {:.1}/{:.1}/{:.1} ms",
                    main_loop.cycle_time.min().as_millis(),
                    main_loop.cycle_time.average().as_millis(),
                    main_loop.cycle_time.max().as_millis()
                ));
            }

            stacked.color(Vec4::new(0.1, 0.7, 1.0, 1.0));
            if let Some(renderer) = renderer {
                stacked.push_down(format!(
                    "FPS: {:.1}/{:.1}/{:.1}. Load: {:.1}/{:.1}/{:.1}%",
                    renderer.fps.min(),
                    renderer.fps.average(),
                    renderer.fps.max(),
                    renderer.load.min() * 100.0,
                    renderer.load.average() * 100.0,
                    renderer.load.max() * 100.0
                ));

                stacked.push_down(format!(
                    "Primitives: {:.1e}/{:.1e}/{:.1e}. Draw Calls: {:.1}/{:.1}/{:.1}",
                    renderer.drawn_primitives.min(),
                    renderer.drawn_primitives.average(),
                    renderer.drawn_primitives.max(),
                    renderer.draw_calls.min(),
                    renderer.draw_calls.average(),
                    renderer.draw_calls.max()
                ));
                stacked.push_down(format!(
                    "Render: {:.1}/{:.1}/{:.1} ms. View: {:.1}/{:.1}/{:.1} ms",
                    renderer.render.min().as_millis(),
                    renderer.render.average().as_millis(),
                    renderer.render.max().as_millis(),
                    renderer.view.min().as_millis(),
                    renderer.view.average().as_millis(),
                    renderer.view.max().as_millis()
                ));
                stacked.push_down(format!(
                    "Events: {:.1}/{:.1}/{:.1}",
                    renderer.events.min().as_millis(),
                    renderer.events.average().as_millis(),
                    renderer.events.max().as_millis()
                ));

                stacked.push_down_str("");
                for (pass_name, pass_time) in &renderer.passes {
                    stacked.push_down(format!(
                        "{}: {:.1}/{:.1}/{:.1} ms",
                        pass_name,
                        pass_time.min().as_millis(),
                        pass_time.average().as_millis(),
                        pass_time.max().as_millis()
                    ));
                }
            }
        }
    }

    ui.input.publish();
}

fn map_font_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    mut ui: Single<&mut UISystem>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) if *id == "martian_regular".into() => {
            let font = hub.get_typed::<Font>("martian_regular".into()).unwrap();
            debug!("Mapped font: {:?}", font);
            ui.font = Some(font.clone());
        }
        _ => {}
    }
}

pub fn setup_ui_system(world: &mut World) -> Output<Vec<UICommand>> {
    let (stream_input, mut stream_output) =
        triple_buffer::<Vec<UICommand>>(&Vec::with_capacity(128));

    world.add_handler(drop_all_assets_handler);
    world.add_handler(map_font_handler);
    world.add_handler(toggle_detailed_handler);
    world.add_handler(main_loop_monitoring_handler);
    world.add_handler(renderer_monitoring_handler);
    world.add_handler(stream_ui_handler);

    let entity = world.spawn();
    world.insert(
        entity,
        UISystem {
            input: stream_input,
            font: None,
            main_loop: None,
            renderer: None,
            viewport: None,
            detailed: false,
        },
    );

    stream_output
}
