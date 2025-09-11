use dawn_assets::hub::AssetInfo;

pub enum ToolAssetsInfoMessage {
    Nothing,
    Refresh,
}

pub fn tool_assets_info(ui: &egui::Context, assets: &Vec<AssetInfo>) -> ToolAssetsInfoMessage {
    egui::Window::new("Assets Information")
        .resizable(true)
        .fade_in(true)
        .fade_out(true)
        .collapsible(true)
        .show(ui, |ui| {
            if ui.button("Refresh").clicked() {};
        });
    ToolAssetsInfoMessage::Nothing
}
