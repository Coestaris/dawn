use crate::logging::format_system_time;
use crate::rendering::devtools::tools::row_height;
use dawn_assets::hub::{AssetInfo, AssetInfoState};
use dawn_assets::AssetType;
use dawn_dac::Manifest;
use egui::Color32;

pub enum ToolAssetsInfoMessage {
    Nothing,
    Refresh,
}

fn pretty_size(size: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;
    const GB: usize = 1024 * MB;

    if size >= GB {
        format!("{:.2} GB", size as f32 / GB as f32)
    } else if size >= MB {
        format!("{:.2} MB", size as f32 / MB as f32)
    } else if size >= KB {
        format!("{:.2} KB", size as f32 / KB as f32)
    } else {
        format!("{} B", size)
    }
}

fn color_type(asset: &AssetInfo) -> Color32 {
    match asset.header.asset_type {
        AssetType::Unknown => Color32::from_rgb(200, 200, 200),
        AssetType::Shader => Color32::from_rgb(200, 100, 100),
        AssetType::Texture => Color32::from_rgb(100, 200, 100),
        AssetType::Audio => Color32::from_rgb(100, 100, 200),
        AssetType::Notes => Color32::from_rgb(200, 200, 100),
        AssetType::Material => Color32::from_rgb(200, 100, 200),
        AssetType::Mesh => Color32::from_rgb(100, 200, 200),
        AssetType::Font => Color32::from_rgb(150, 150, 250),
        AssetType::Dictionary => Color32::from_rgb(250, 150, 150),
        AssetType::Blob => Color32::from_rgb(150, 250, 150),
    }
}

fn color_state(asset: &AssetInfo) -> Color32 {
    match asset.state {
        AssetInfoState::Empty => Color32::from_rgb(200, 200, 200),
        AssetInfoState::IR(_) => Color32::from_rgb(200, 100, 100),
        AssetInfoState::Loaded { .. } => Color32::from_rgb(100, 200, 100),
    }
}

pub fn tool_assets_info(
    ui: &egui::Context,
    assets: &Vec<AssetInfo>,
    manifest: Option<&Manifest>,
) -> ToolAssetsInfoMessage {
    let mut result = ToolAssetsInfoMessage::Nothing;
    egui::Window::new("📄 Assets Information")
        .resizable(true)
        .fade_in(true)
        .fade_out(true)
        .collapsible(true)
        .show(ui, |ui| {
            if ui.button("Refresh").clicked() {
                result = ToolAssetsInfoMessage::Refresh;
            };

            if let Some(manifest) = manifest {
                ui.collapsing("Assets Manifest", |ui| {
                    if let Some(author) = &manifest.author {
                        ui.label(format!("Author: {}", author));
                    }
                    if let Some(description) = &manifest.description {
                        ui.label(format!("Description: {}", description));
                    }
                    if let Some(version) = &manifest.version {
                        ui.label(format!("Version: {}", version));
                    }
                    if let Some(license) = &manifest.license {
                        ui.label(format!("License: {}", license));
                    }
                    ui.label(format!(
                        "Tool: {} v{}",
                        manifest.tool, manifest.tool_version
                    ));
                    ui.label(format!(
                        "Created: {}",
                        format_system_time(manifest.created).unwrap()
                    ));
                    ui.label(format!("Read mode: {:?}", manifest.read_mode));
                    ui.label(format!(
                        "Checksum algorithm: {:?}",
                        manifest.checksum_algorithm
                    ));
                    ui.label(format!("Total assets: {}", manifest.headers.len()));
                });
            }

            if assets.is_empty() {
                ui.label("No assets loaded.");
                return;
            }

            let text_height = row_height();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .column(egui_extras::Column::auto().at_least(500.0))
                    .column(egui_extras::Column::remainder().at_least(70.0))
                    .column(egui_extras::Column::remainder().at_least(70.0))
                    .column(egui_extras::Column::remainder().at_least(60.0))
                    .column(egui_extras::Column::remainder().at_least(60.0))
                    .column(egui_extras::Column::remainder().at_least(60.0))
                    .header(text_height, |mut header| {
                        header.col(|ui| {
                            ui.strong("Name (hover me)");
                        });
                        header.col(|ui| {
                            ui.strong("Type");
                        });
                        header.col(|ui| {
                            ui.strong("State");
                        });
                        header.col(|ui| {
                            ui.strong("Ref count");
                        });
                        header.col(|ui| {
                            ui.strong("RAM");
                        });
                        header.col(|ui| {
                            ui.strong("VRAM");
                        });
                    })
                    .body(|mut body| {
                        for asset in assets {
                            body.row(text_height, |mut row| {
                                row.col(|ui| {
                                    ui.label(asset.header.id.as_str()).on_hover_text(
                                        format!(
                                            "ID: {}\nType: {}\nChecksum: {:?}\nDependencies: {:?}\nTags: {:?}\nAuthor: {:?}\nLicense: {:?}",
                                            asset.header.id.as_str(),
                                            asset.header.asset_type.as_str(),
                                            asset.header.checksum,
                                            asset.header.dependencies,
                                            asset.header.tags,
                                            asset.header.author,
                                            asset.header.license,
                                        )
                                    );
                                    // Add tooltip with detailed info
                                });
                                row.col(|ui| {
                                    ui.colored_label(
                                        color_type(asset),
                                        asset.header.asset_type.as_str(),
                                    );
                                });
                                row.col(|ui| {
                                    ui.colored_label(color_state(asset), asset.state.as_str());
                                });
                                row.col(|ui| match asset.state.as_ref_count() {
                                    Some(count) => {
                                        ui.label(format!("{}", count));
                                    }
                                    None => {
                                        ui.label("-");
                                    }
                                });
                                row.col(|ui| match asset.state.as_ram_usage() {
                                    Some(size) => {
                                        ui.label(pretty_size(size));
                                    }
                                    None => {
                                        ui.label("-");
                                    }
                                });
                                row.col(|ui| match asset.state.as_vram_usage() {
                                    Some(size) => {
                                        ui.label(pretty_size(size));
                                    }
                                    None => {
                                        ui.label("-");
                                    }
                                });
                            });
                        }
                    });
            });
        });

    result
}
