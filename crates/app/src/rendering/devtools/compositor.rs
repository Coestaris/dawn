use crate::assets::reader::ReaderBackend;
use crate::devtools::{
    DevtoolsRendererConnection, DevtoolsToRendererMessage, DevtoolsToWorldMessage, SunlightControl,
};
use crate::rendering::config::RenderingConfig;
use crate::rendering::devtools::tools::about::tool_about;
use crate::rendering::devtools::tools::assets_info::{tool_assets_info, ToolAssetsInfoMessage};
use crate::rendering::devtools::tools::controls::tool_controls;
use crate::rendering::devtools::tools::rendering_settings::{
    tool_rendering_settings, ToolRenderingSettingsMessage,
};
use crate::rendering::devtools::tools::rendering_stat::tool_rendering_stat;
use crate::rendering::devtools::tools::world_stat::tool_world_stat;
use crate::world::devtools::WorldStatistics;
use build_info::BuildInfo;
use dawn_assets::hub::AssetInfo;
use dawn_dac::Manifest;
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::gl::probe::OpenGLInfo;
use dawn_graphics::renderer::RendererMonitorEvent;
use std::sync::Arc;

pub(crate) struct Compositor {
    connection: DevtoolsRendererConnection,
    bi: BuildInfo,
    config: RenderingConfig,
    reader_backend: Arc<dyn ReaderBackend>,
    manifest: Option<Manifest>,

    display_world_stat: bool,
    display_rendering_stat: bool,
    display_rendering_settings: bool,
    display_assets_infos: bool,
    display_about: bool,
    display_controls: bool,

    gl_info: Option<OpenGLInfo>,
    assets_infos: Vec<AssetInfo>,
    world_stat: Option<(WorldLoopMonitorEvent, WorldStatistics)>,
    rendering_stat: Option<RendererMonitorEvent>,
    sunlight_control: SunlightControl,
}

impl Compositor {
    pub fn new(
        connection: DevtoolsRendererConnection,
        config: RenderingConfig,
        bi: BuildInfo,
        reader_backend: Arc<dyn ReaderBackend>,
    ) -> Self {
        Self {
            connection,
            bi,
            reader_backend,
            config,
            display_world_stat: false,
            display_rendering_stat: false,
            display_rendering_settings: false,
            display_assets_infos: false,
            display_about: false,
            display_controls: false,
            gl_info: None,
            assets_infos: vec![],
            world_stat: None,
            rendering_stat: None,
            sunlight_control: SunlightControl::default(),
            manifest: None,
        }
    }

    pub fn update_gl_info(&mut self, info: OpenGLInfo) {
        self.gl_info = Some(info);
    }

    pub fn before_frame(&mut self) {
        // Handle incoming messages if needed
        while let Ok(message) = self.connection.receiver.try_recv() {
            match message {
                DevtoolsToRendererMessage::WorldMonitor(me, ws) => {
                    self.world_stat = Some((me, ws));
                }
                DevtoolsToRendererMessage::RendererMonitor(re) => {
                    self.rendering_stat = Some(re);
                }
                DevtoolsToRendererMessage::AssetsEnumerated(assets) => {
                    self.assets_infos = assets;
                }
            }
        }
    }

    pub fn run(&mut self, ui: &egui::Context) {
        // Create a toolbar window
        let fill = ui.style().visuals.window_fill();
        egui::TopBottomPanel::bottom("toolbar")
            .frame(egui::Frame::default().inner_margin(5.0).fill(fill))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("♂");
                    ui.separator();

                    // Output: FPS: xx, Frame time: xx ms. Tools:
                    if let Some(rs) = &self.rendering_stat {
                        ui.label(format!("FPS: {:.2}", rs.fps.average(),));
                    } else {
                        ui.label("FPS: N/A");
                    }

                    ui.separator();
                    ui.label("Tools:");

                    fn highlighted_button(ui: &mut egui::Ui, label: &str, active: &mut bool) {
                        let btn = ui.button(label);
                        let btn = if *active { btn.highlight() } else { btn };
                        if btn.clicked() {
                            *active = !*active;
                        }
                    }

                    highlighted_button(ui, "About", &mut self.display_about);
                    highlighted_button(ui, "Controls", &mut self.display_controls);
                    highlighted_button(ui, "World Statistics", &mut self.display_world_stat);
                    highlighted_button(
                        ui,
                        "Rendering Statistics",
                        &mut self.display_rendering_stat,
                    );
                    highlighted_button(
                        ui,
                        "Rendering Settings",
                        &mut self.display_rendering_settings,
                    );
                    highlighted_button(ui, "Assets Info", &mut self.display_assets_infos);

                    ui.separator();
                    ui.add_space(ui.available_width() - 26.0);
                    ui.separator();
                    ui.label("♂");
                });
            });

        if self.display_world_stat {
            if let Some((me, ws)) = &self.world_stat {
                tool_world_stat(ui, me, ws);
            }
        }
        if self.display_rendering_stat {
            if let Some(rs) = &self.rendering_stat {
                tool_rendering_stat(ui, rs);
            }
        }
        if self.display_rendering_settings {
            match tool_rendering_settings(ui, &mut self.config, &mut self.sunlight_control) {
                ToolRenderingSettingsMessage::Nothing => {}
                ToolRenderingSettingsMessage::ControlSunlight => {
                    let _ = self
                        .connection
                        .sender
                        .send(DevtoolsToWorldMessage::ControlSunlight(
                            self.sunlight_control.clone(),
                        ));
                }
            }
        }
        if self.display_assets_infos {
            match tool_assets_info(ui, &self.assets_infos, self.manifest.as_ref()) {
                ToolAssetsInfoMessage::Nothing => {}
                ToolAssetsInfoMessage::Refresh => {
                    self.manifest = self.reader_backend.enumerate().ok();
                    let _ = self
                        .connection
                        .sender
                        .send(DevtoolsToWorldMessage::EnumerateAssets);
                }
            }
        }
        if self.display_about {
            tool_about(ui, &self.bi, self.gl_info.as_ref());
        }
        if self.display_controls {
            tool_controls(ui);
        }
    }
}
