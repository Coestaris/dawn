use crate::ui::{UIRendererConnection, UIToRendererMessage, UIToWorldMessage};
use crate::world::ui::WorldStatistics;
use dawn_assets::hub::{AssetInfo, AssetInfoState};
use dawn_assets::AssetType;
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::renderer::RendererMonitorEvent;
use imgui::sys::{ImGuiTableColumnFlags, ImGuiTableFlags, ImGuiTableRowFlags, ImVec2};
use imgui::Ui;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundingBoxMode {
    Disabled,
    AABB,
    AABBHonorDepth,
    OBB,
    OBBHonorDepth,
}

impl BoundingBoxMode {
    pub fn items() -> [&'static str; 5] {
        [
            BoundingBoxMode::Disabled.as_str(),
            BoundingBoxMode::AABB.as_str(),
            BoundingBoxMode::AABBHonorDepth.as_str(),
            BoundingBoxMode::OBB.as_str(),
            BoundingBoxMode::OBBHonorDepth.as_str(),
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BoundingBoxMode::Disabled => "Disabled",
            BoundingBoxMode::AABB => "AABB",
            BoundingBoxMode::AABBHonorDepth => "AABB (Honor Depth)",
            BoundingBoxMode::OBB => "OBB",
            BoundingBoxMode::OBBHonorDepth => "OBB (Honor Depth)",
        }
    }
}

impl From<usize> for BoundingBoxMode {
    fn from(value: usize) -> Self {
        match value {
            0 => BoundingBoxMode::Disabled,
            1 => BoundingBoxMode::AABB,
            2 => BoundingBoxMode::AABBHonorDepth,
            3 => BoundingBoxMode::OBB,
            4 => BoundingBoxMode::OBBHonorDepth,
            _ => {
                panic!("Unknown bounding box mode index {}", value);
            }
        }
    }
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTexture {
    Final,
    AlbedoMetallic,
    Normal,
    PBR,
}

impl OutputTexture {
    pub fn items() -> [&'static str; 4] {
        [
            OutputTexture::Final.as_str(),
            OutputTexture::AlbedoMetallic.as_str(),
            OutputTexture::Normal.as_str(),
            OutputTexture::PBR.as_str(),
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            OutputTexture::Final => "Final",
            OutputTexture::AlbedoMetallic => "Albedo + Metallic",
            OutputTexture::Normal => "Normal",
            OutputTexture::PBR => "PBR",
        }
    }
}

impl From<usize> for OutputTexture {
    fn from(value: usize) -> Self {
        match value {
            0 => OutputTexture::Final,
            1 => OutputTexture::AlbedoMetallic,
            2 => OutputTexture::Normal,
            3 => OutputTexture::PBR,
            _ => {
                panic!("Unknown output texture index {}", value);
            }
        }
    }
}

pub struct RenderingConfigInner {
    pub wireframe: bool,
    pub output_texture: OutputTexture,
    pub bounding_box_mode: BoundingBoxMode,
    pub show_gizmos: bool,
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
            output_texture: OutputTexture::Final,
            bounding_box_mode: BoundingBoxMode::Disabled,
            show_gizmos: false,
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
    world_statistics: Option<WorldStatistics>,
    assets_info: Option<Vec<AssetInfo>>,

    run: bool,
    first_run: bool,
}

impl UI {
    pub fn create_context() -> imgui::Context {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None); // Disable imgui.ini
        let style = imgui.style_mut();
        style.window_rounding = 5.0;
        style.frame_rounding = 5.0;
        style.scrollbar_rounding = 5.0;
        style.grab_rounding = 5.0;
        style.use_dark_colors();
        style.colors[imgui::StyleColor::WindowBg as usize][3] = 0.7;

        // Make default font a bit larger
        // TODO: Load a better font
        let font_size = 15.0;
        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

        imgui.io_mut().config_flags |= imgui::ConfigFlags::DOCKING_ENABLE;

        imgui
    }

    pub fn new(config: RenderingConfig, connection: UIRendererConnection) -> Self {
        Self {
            connection,
            config,
            renderer_monitor_event: None,
            world_monitor_event: None,
            world_statistics: None,
            assets_info: None,
            run: true,
            first_run: true,
        }
    }

    pub fn before_frame(
        &mut self,
        imgui: &mut imgui::Context,
        _window: &winit::window::Window,
    ) -> bool {
        if let Ok(message) = self.connection.receiver.try_recv() {
            match message {
                UIToRendererMessage::RendererMonitor(event) => {
                    self.renderer_monitor_event = Some(event);
                }
                UIToRendererMessage::WorldMonitor(event, statistics) => {
                    self.world_monitor_event = Some(event);
                    self.world_statistics = Some(statistics);
                }
                UIToRendererMessage::AssetsEnumerated(assets) => {
                    self.assets_info = Some(assets);
                }
                UIToRendererMessage::SetUIFont(raw_ttf) => {
                    let font_size = 15.0;
                    imgui.fonts().clear();
                    imgui.fonts().add_font(&[imgui::FontSource::TtfData {
                        data: &raw_ttf,
                        size_pixels: font_size,
                        config: None,
                    }]);
                    imgui.io_mut().font_global_scale = 1.0;

                    return true;
                }
            }
        }

        false
    }

    pub fn render(&mut self, ui: &mut Ui) {
        let dock = ui.dockspace_over_main_viewport();

        const CONTROLS_NAME: &str = "Controls";
        const WORLD_MONITOR_NAME: &str = "World Monitor";
        const RENDERER_MONITOR_NAME: &str = "Renderer Monitor";
        const RENDERING_SETTINGS_NAME: &str = "Rendering Settings";
        const ASSETS_BROWSER_NAME: &str = "Assets Browser";

        // Show FPS
        if let ((Some(renderer_event), Some(world_event), Some(world_statistics))) = (
            &self.renderer_monitor_event,
            &self.world_monitor_event,
            &self.world_statistics,
        ) {
            ui.window(ASSETS_BROWSER_NAME)
                .size([300.0, 400.0], imgui::Condition::FirstUseEver)
                .position(
                    [
                        ui.io().display_size[0] - 10.0,
                        ui.io().display_size[1] - 10.0,
                    ],
                    imgui::Condition::FirstUseEver,
                )
                .always_auto_resize(true)
                .build(|| {
                    if ui.button("Enumerate Assets") {
                        let _ = self
                            .connection
                            .sender
                            .send(UIToWorldMessage::EnumerateAssets);
                    }
                    ui.separator();

                    // Build a table with 3 columns: Name, Type, State, Space Used ram, Space used vram
                    if let Some(assets) = &self.assets_info {
                        unsafe {
                            let cstr = std::ffi::CString::new(ASSETS_BROWSER_NAME).unwrap();
                            imgui::sys::igBeginTable(
                                cstr.as_ptr(),
                                6,
                                (imgui::sys::ImGuiTableFlags_SizingFixedFit
                                    | imgui::sys::ImGuiTableFlags_RowBg
                                    | imgui::sys::ImGuiTableFlags_BordersInnerV
                                    | imgui::sys::ImGuiTableFlags_Resizable
                                    | imgui::sys::ImGuiTableFlags_ScrollY)
                                    as ImGuiTableFlags,
                                ImVec2::new(0.0, 0.0),
                                0.0,
                            );

                            let cstr = std::ffi::CString::new("ID").unwrap();
                            imgui::sys::igTableSetupColumn(
                                cstr.as_ptr() as *const i8,
                                imgui::sys::ImGuiTableColumnFlags_WidthStretch
                                    as ImGuiTableColumnFlags,
                                0.0,
                                0,
                            );
                            let cstr = std::ffi::CString::new("Type").unwrap();
                            imgui::sys::igTableSetupColumn(
                                cstr.as_ptr() as *const i8,
                                0 as ImGuiTableColumnFlags,
                                0.0,
                                1,
                            );
                            let cstr = std::ffi::CString::new("State").unwrap();
                            imgui::sys::igTableSetupColumn(
                                cstr.as_ptr() as *const i8,
                                0 as ImGuiTableColumnFlags,
                                0.0,
                                2,
                            );
                            let cstr = std::ffi::CString::new("Ref Count").unwrap();
                            imgui::sys::igTableSetupColumn(
                                cstr.as_ptr() as *const i8,
                                0 as ImGuiTableColumnFlags,
                                0.0,
                                3,
                            );
                            let cstr = std::ffi::CString::new("RAM").unwrap();
                            imgui::sys::igTableSetupColumn(
                                cstr.as_ptr() as *const i8,
                                0 as ImGuiTableColumnFlags,
                                0.0,
                                4,
                            );
                            let cstr = std::ffi::CString::new("VRAM").unwrap();
                            imgui::sys::igTableSetupColumn(
                                cstr.as_ptr() as *const i8,
                                0 as ImGuiTableColumnFlags,
                                0.0,
                                5,
                            );

                            imgui::sys::igTableHeadersRow();
                            for asset in assets {
                                imgui::sys::igTableNextRow(
                                    imgui::sys::ImGuiTableRowFlags_None as ImGuiTableRowFlags,
                                    0.0,
                                );
                                imgui::sys::igTableSetColumnIndex(0);
                                ui.text(&asset.header.id.as_str());

                                fn color_type(asset: &AssetInfo) -> [f32; 4] {
                                    match asset.header.asset_type {
                                        AssetType::Unknown => [0.5, 0.5, 0.5, 1.0],
                                        AssetType::Shader => [0.8, 0.8, 0.2, 1.0],
                                        AssetType::Texture => [0.2, 0.8, 0.2, 1.0],
                                        AssetType::Audio => [0.2, 0.2, 0.8, 1.0],
                                        AssetType::Notes => [0.8, 0.2, 0.8, 1.0],
                                        AssetType::Material => [0.2, 0.8, 0.8, 1.0],
                                        AssetType::Mesh => [0.8, 0.5, 0.2, 1.0],
                                        AssetType::Font => [0.5, 0.2, 0.8, 1.0],
                                        AssetType::Dictionary => [0.2, 0.5, 0.8, 1.0],
                                        AssetType::Blob => [0.8, 0.2, 0.5, 1.0],
                                    }
                                }

                                fn color_state(asset: &AssetInfo) -> [f32; 4] {
                                    match asset.state {
                                        AssetInfoState::Empty => [0.5, 0.5, 0.5, 1.0],
                                        AssetInfoState::IR(_) => [0.8, 0.8, 0.2, 1.0],
                                        AssetInfoState::Loaded { .. } => [0.2, 0.8, 0.2, 1.0],
                                    }
                                }

                                imgui::sys::igTableSetColumnIndex(1);
                                ui.text_colored(
                                    color_type(asset),
                                    format!("{:?}", asset.header.asset_type),
                                );
                                imgui::sys::igTableSetColumnIndex(2);
                                ui.text_colored(color_state(asset), asset.state.as_str());

                                fn pretty_size(bytes: usize) -> String {
                                    const KB: f32 = 1024.0;
                                    const MB: f32 = KB * 1024.0;
                                    const GB: f32 = MB * 1024.0;

                                    let bytes_f = bytes as f32;
                                    if bytes_f >= GB {
                                        format!("{:.2} GB", bytes_f / GB)
                                    } else if bytes_f >= MB {
                                        format!("{:.2} MB", bytes_f / MB)
                                    } else if bytes_f >= KB {
                                        format!("{:.2} KB", bytes_f / KB)
                                    } else {
                                        format!("{} B", bytes)
                                    }
                                }

                                match &asset.state {
                                    AssetInfoState::Loaded { usage, rc } => {
                                        imgui::sys::igTableSetColumnIndex(3);
                                        ui.text(&format!("{}", rc));
                                        imgui::sys::igTableSetColumnIndex(4);
                                        ui.text(&format!("{}", pretty_size(usage.ram)));
                                        imgui::sys::igTableSetColumnIndex(5);
                                        ui.text(&format!("{}", pretty_size(usage.vram)));
                                    }
                                    _ => {
                                        imgui::sys::igTableSetColumnIndex(3);
                                        ui.text("-");
                                        imgui::sys::igTableSetColumnIndex(4);
                                        ui.text("-");
                                        imgui::sys::igTableSetColumnIndex(5);
                                        ui.text("-");
                                    }
                                }
                            }
                            imgui::sys::igEndTable();
                        }
                    } else {
                        ui.text("No assets enumerated.");
                    }
                });

            ui.window(WORLD_MONITOR_NAME)
                .position([10.0, 10.0], imgui::Condition::FirstUseEver)
                .always_auto_resize(true)
                .build(|| {
                    const WORLD_COLOR: [f32; 4] = [1.0, 0.7, 0.1, 1.0];
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
                        WORLD_COLOR,
                        format!("Entities: {}", world_statistics.entities),
                    );
                    ui.text_colored(
                        WORLD_COLOR,
                        format!("Drawables: {}", world_statistics.drawables),
                    );
                    ui.text_colored(
                        WORLD_COLOR,
                        format!("Point Lights: {}", world_statistics.point_lights),
                    );
                    ui.text_colored(
                        WORLD_COLOR,
                        format!("Spot Lights: {}", world_statistics.spot_lights),
                    );
                    ui.text_colored(
                        WORLD_COLOR,
                        format!("Sun Lights: {}", world_statistics.sun_lights),
                    );
                    ui.text_colored(
                        WORLD_COLOR,
                        format!("Area Lights: {}", world_statistics.area_lights),
                    );
                });

            ui.window(RENDERER_MONITOR_NAME)
                .position(
                    [10.0, ui.window_size()[1] + 10.0],
                    imgui::Condition::FirstUseEver,
                )
                .always_auto_resize(true)
                .build(|| {
                    const RENDERING_COLOR: [f32; 4] = [0.1, 0.7, 1.0, 1.0];
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

        ui.window(RENDERING_SETTINGS_NAME)
            .size([300.0, 200.0], imgui::Condition::FirstUseEver)
            .position_pivot([1.0, 1.0])
            .position([10.0, 0.0], imgui::Condition::FirstUseEver)
            .always_auto_resize(true)
            .build(|| {
                let mut config = self.config.borrow_mut();

                ui.checkbox("Wireframe Mode", &mut config.wireframe);
                ui.checkbox("Show Gizmos", &mut config.show_gizmos);
                ui.separator();

                let items = OutputTexture::items();
                let mut selected = config.output_texture.as_str();
                let mut selected_index = config.output_texture as usize;
                if let Some(cb) = ui.begin_combo("Output", selected) {
                    for (i, &cur) in items.iter().enumerate() {
                        if selected == cur {
                            // Auto-scroll to selected item
                            ui.set_item_default_focus();
                        }
                        // Create a "selectable"
                        let clicked = ui.selectable_config(cur).selected(selected == cur).build();
                        // When item is clicked, store it
                        if clicked {
                            selected = cur;
                            selected_index = i;
                        }
                    }
                }
                config.output_texture = OutputTexture::from(selected_index);

                let items = BoundingBoxMode::items();
                let mut selected = config.bounding_box_mode.as_str();
                let mut selected_index = config.bounding_box_mode as usize;
                if let Some(cb) = ui.begin_combo("Bounding Box", selected) {
                    for (i, &cur) in items.iter().enumerate() {
                        if selected == cur {
                            // Auto-scroll to selected item
                            ui.set_item_default_focus();
                        }
                        // Create a "selectable"
                        let clicked = ui.selectable_config(cur).selected(selected == cur).build();
                        // When item is clicked, store it
                        if clicked {
                            selected = cur;
                            selected_index = i;
                        }
                    }
                }
                config.bounding_box_mode = BoundingBoxMode::from(selected_index);
            });

        ui.window(CONTROLS_NAME)
            .position(
                [10.0, ui.io().display_size[1] - 10.0],
                imgui::Condition::FirstUseEver,
            )
            .position_pivot([0.0, 1.0])
            .always_auto_resize(true)
            .build(|| {
                ui.text("ESC - Quit");
                ui.text("F1 - Toggle UI");
                ui.text("F5 - Reload Assets");
                ui.text("WASD/Shift/Space - Move Camera");
                ui.text("Right Mouse Drag - Rotate Camera");
            });

        if self.first_run {
            self.first_run = false;

            unsafe {
                // imgui::sys::igDockBuilderRemoveNode(dock);
                // imgui::sys::igDockBuilderAddNode(dock, imgui::sys::ImGuiDockNodeFlags_None as imgui::sys::ImGuiDockNodeFlags);
                imgui::sys::igDockBuilderSetNodeSize(
                    dock,
                    ImVec2::new(ui.io().display_size[0] + 2.0, ui.io().display_size[1] + 2.0),
                );

                let mut left: imgui::sys::ImGuiID = 0;
                let mut right: imgui::sys::ImGuiID = 0;
                imgui::sys::igDockBuilderSplitNode(
                    dock,
                    imgui::sys::ImGuiDir_Left,
                    0.3,
                    &mut left as *mut imgui::sys::ImGuiID,
                    &mut right as *mut imgui::sys::ImGuiID,
                );

                // Split left into top and bottom
                let mut part1: imgui::sys::ImGuiID = 0;
                let mut middle: imgui::sys::ImGuiID = 0;
                imgui::sys::igDockBuilderSplitNode(
                    left,
                    imgui::sys::ImGuiDir_Up,
                    0.4,
                    &mut part1 as *mut imgui::sys::ImGuiID,
                    &mut middle as *mut imgui::sys::ImGuiID,
                );
                // Split right into top and bottom
                let mut part2: imgui::sys::ImGuiID = 0;
                let mut part3: imgui::sys::ImGuiID = 0;
                imgui::sys::igDockBuilderSplitNode(
                    middle,
                    imgui::sys::ImGuiDir_Up,
                    0.6,
                    &mut part2 as *mut imgui::sys::ImGuiID,
                    &mut part3 as *mut imgui::sys::ImGuiID,
                );

                let cstr = std::ffi::CString::new(CONTROLS_NAME).unwrap();
                imgui::sys::igDockBuilderDockWindow(cstr.as_ptr(), part3);
                let cstr = std::ffi::CString::new(RENDERING_SETTINGS_NAME).unwrap();
                imgui::sys::igDockBuilderDockWindow(cstr.as_ptr(), part3);
                let cstr = std::ffi::CString::new(ASSETS_BROWSER_NAME).unwrap();
                imgui::sys::igDockBuilderDockWindow(cstr.as_ptr(), part3);
                let cstr = std::ffi::CString::new(WORLD_MONITOR_NAME).unwrap();
                imgui::sys::igDockBuilderDockWindow(cstr.as_ptr(), part2);
                let cstr = std::ffi::CString::new(RENDERER_MONITOR_NAME).unwrap();
                imgui::sys::igDockBuilderDockWindow(cstr.as_ptr(), part1);

                imgui::sys::igDockBuilderFinish(dock);
            }
        }
    }
}
