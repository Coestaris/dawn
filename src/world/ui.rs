use crate::world::asset::DropAllAssetsEvent;
use build_info::semver::Op;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::TypedAsset;
use dawn_ecs::events::InterSyncEvent;
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::gl::font::Font;
use dawn_graphics::renderer::{InputEvent, RendererMonitorEvent};
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::Single;
use evenio::world::World;
use glam::{UVec2, Vec2, Vec4};
use log::{debug, info};
use std::cell::UnsafeCell;
use std::sync::Arc;
use triple_buffer::{triple_buffer, Input, Output};
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{Key, NamedKey};

#[derive(Debug, Clone)]
pub struct Style {
    pub font: TypedAsset<Font<'static>>,
    pub scale: f32,
}

#[derive(Debug, Clone)]
pub enum UICommand {
    ApplyStyle(Style),
    ChangeColor(Vec4),
    StaticText(Vec2, &'static str),
    Text(Vec2, String),
    Box(Vec2, Vec2), // position, dimensions
}

#[derive(Component)]
struct UISystem {
    writer: Input<Vec<UICommand>>,
    font: Option<TypedAsset<Font<'static>>>,
    main_loop: Option<WorldLoopMonitorEvent>,
    renderer: Option<RendererMonitorEvent>,
    viewport: Option<UVec2>,
    detailed: bool,
}

pub struct UIReader {
    // Oh god why. I'll fix this later
    stream: Arc<UnsafeCell<Output<Vec<UICommand>>>>,
}

unsafe impl Send for UIReader {}
unsafe impl Sync for UIReader {}

impl UIReader {
    pub fn bridge() -> (Input<Vec<UICommand>>, Self) {
        let (input, output) = triple_buffer::<Vec<UICommand>>(&Vec::with_capacity(128));
        (
            input,
            Self {
                stream: Arc::new(UnsafeCell::new(output)),
            },
        )
    }

    pub fn get_data_mut<'a>(&self) -> &'a mut Vec<UICommand> {
        unsafe { self.stream.get().as_mut().unwrap().output_buffer_mut() }
    }

    pub fn get_data<'a>(&self) -> &'a Vec<UICommand> {
        unsafe { self.stream.get().as_ref().unwrap().peek_output_buffer() }
    }

    pub fn update(&self) {
        unsafe { self.stream.get().as_mut().unwrap().update() };
    }
}

impl Clone for UIReader {
    fn clone(&self) -> Self {
        Self {
            stream: self.stream.clone(),
        }
    }
}

fn toggle_detailed_handler(r: Receiver<InputEvent>, mut ui: Single<&mut UISystem>) {
    match &r.event.0 {
        WindowEvent::Resized(size) => {
            ui.viewport = Some(UVec2::new(size.width, size.height));
        }
        WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    logical_key: key,
                    state: ElementState::Released,
                    ..
                },
            ..
        } => match key.as_ref() {
            Key::Named(NamedKey::F1) => {
                ui.detailed = !ui.detailed;
                debug!("Toggled detailed UI: {}", ui.detailed);
            }
            _ => {}
        },

        _ => {}
    }
}

fn drop_all_assets_handler(r: Receiver<DropAllAssetsEvent>, mut ui: Single<&mut UISystem>) {
    debug!("Dropping all UI assets");
    ui.font = None;

    // Flush the content of the input buffer
    // The triple buffer must be cleared... you guessed it... three times
    // It's ugly, but it works
    let vec = ui.writer.input_buffer_mut();
    vec.clear();
    ui.writer.publish();
    let vec = ui.writer.input_buffer_mut();
    vec.clear();
    ui.writer.publish();
    let vec = ui.writer.input_buffer_mut();
    vec.clear();
    ui.writer.publish();
}

fn main_loop_monitoring_handler(r: Receiver<WorldLoopMonitorEvent>, mut ui: Single<&mut UISystem>) {
    ui.main_loop = Some(r.event.clone());
}

fn renderer_monitoring_handler(r: Receiver<RendererMonitorEvent>, mut ui: Single<&mut UISystem>) {
    ui.renderer = Some(r.event.clone());
}

struct Stacked<'a> {
    position: Vec2,
    output: &'a mut Vec<UICommand>,
    style: &'a Style,
}

impl<'a> Stacked<'a> {
    pub fn new(position: Vec2, output: &'a mut Vec<UICommand>, style: &'a Style) -> Self {
        Self {
            position,
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
    let viewport = ui.viewport.unwrap_or(UVec2::new(800, 600));
    let detailed = ui.detailed;

    let vec = ui.writer.input_buffer_mut();
    vec.clear();

    if let Some(font) = font {
        let style = Style {
            font: font.clone(),
            scale: 0.5,
        };
        vec.push(UICommand::ApplyStyle(style.clone()));

        if !detailed {
            {
                let stacked = &mut Stacked::new(Vec2::new(5.0, viewport.y as f32), vec, &style);
                // Change color to yellow
                stacked.color(Vec4::new(1.0, 1.0, 0.0, 1.0));
                stacked.push_up_str("Press F1 for detailed info");
            }

            let stacked = &mut Stacked::new(Vec2::new(5.0, 5.0), vec, &style);
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
                let stacked = &mut Stacked::new(Vec2::new(5.0, viewport.y as f32), vec, &style);
                // Change color to yellow
                stacked.color(Vec4::new(1.0, 1.0, 0.0, 1.0));
                stacked.push_up_str("WASD/Shift/Space + Mouse Drag - Move camera");
                stacked.push_up_str("F11  - Toggle fullscreen");
                stacked.push_up_str("F5   - Refresh assets");
                stacked.push_up_str("F4   - Toggle Bounding Boxes");
                stacked.push_up_str("F3   - Toggle wireframe");
                stacked.push_up_str("F1   - Toggle detailed info");
                stacked.push_up_str("ESC  - Quit");
                stacked.push_up_str("Controls: ");
            }

            let stacked = &mut Stacked::new(Vec2::new(5.0, 5.0), vec, &style);
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
                    "Primitives: {:.1e}/{:.1e}/{:.1e}. Draw Calls: {:.1e}/{:.1e}/{:.1e}",
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

    ui.writer.publish();
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

pub fn setup_ui_system(world: &mut World, ui_writer: Input<Vec<UICommand>>) {
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
            writer: ui_writer,
            font: None,
            main_loop: None,
            renderer: None,
            viewport: None,
            detailed: false,
        },
    );
}
