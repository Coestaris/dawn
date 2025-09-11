use crate::devtools::{
    DevtoolsRendererConnection, DevtoolsToRendererMessage, DevtoolsToWorldMessage,
};
use crate::rendering::config::RenderingConfig;
use crate::rendering::devtools::tools::about::tool_about;
use crate::rendering::devtools::tools::assets_info::{tool_assets_info, ToolAssetsInfoMessage};
use crate::rendering::devtools::tools::controls::tool_controls;
use crate::rendering::devtools::tools::rendering_settings::tool_rendering_settings;
use crate::rendering::devtools::tools::rendering_stat::tool_rendering_stat;
use crate::rendering::devtools::tools::world_stat::tool_world_stat;
use crate::world::devtools::WorldStatistics;
use dawn_assets::hub::AssetInfo;
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::renderer::RendererMonitorEvent;

pub(crate) struct Compositor {
    connection: DevtoolsRendererConnection,
    config: RenderingConfig,

    display_world_stat: bool,
    display_rendering_stat: bool,
    display_rendering_settings: bool,
    display_assets_infos: bool,
    display_about: bool,
    display_controls: bool,

    assets_infos: Vec<AssetInfo>,
    world_stat: Option<(WorldLoopMonitorEvent, WorldStatistics)>,
    rendering_stat: Option<RendererMonitorEvent>,
}

impl Compositor {
    pub fn new(connection: DevtoolsRendererConnection, config: RenderingConfig) -> Self {
        Self {
            connection,
            config,
            display_world_stat: false,
            display_rendering_stat: false,
            display_rendering_settings: false,
            display_assets_infos: false,
            display_about: false,
            display_controls: false,
            assets_infos: vec![],
            world_stat: None,
            rendering_stat: None,
        }
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
        egui::TopBottomPanel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                // Output: FPS: xx, Frame time: xx ms. Tools:
                if let Some(rs) = &self.rendering_stat {
                    ui.label(format!("FPS: {:.2}", rs.fps.average(),));
                } else {
                    ui.label("FPS: N/A");
                }

                ui.separator();
                ui.label("Tools:");

                if ui.button("About").clicked() {
                    self.display_about = !self.display_about;
                }
                if ui.button("Controls").clicked() {
                    self.display_controls = !self.display_controls;
                }
                if ui.button("World Stats").clicked() {
                    self.display_world_stat = !self.display_world_stat;
                }
                if ui.button("Rendering Stats").clicked() {
                    self.display_rendering_stat = !self.display_rendering_stat;
                }
                if ui.button("Rendering Settings").clicked() {
                    self.display_rendering_settings = !self.display_rendering_settings;
                }
                if ui.button("Assets Info").clicked() {
                    self.display_assets_infos = !self.display_assets_infos;
                }
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
            tool_rendering_settings(ui, &mut self.config);
        }
        if self.display_assets_infos {
            match tool_assets_info(ui, &self.assets_infos) {
                ToolAssetsInfoMessage::Nothing => {}
                ToolAssetsInfoMessage::Refresh => {
                    let _ = self
                        .connection
                        .sender
                        .send(DevtoolsToWorldMessage::EnumerateAssets);
                }
            }
        }
        if self.display_about {
            tool_about(ui);
        }
        if self.display_controls {
            tool_controls(ui);
        }
    }
}
