use dawn_assets::hub::{AssetInfo,};

pub enum ToolAssetsInfoMessage {
    Nothing,
    Refresh,
}

pub fn tool_assets_info(ui: &egui::Context, assets: &Vec<AssetInfo>) -> ToolAssetsInfoMessage {
    ToolAssetsInfoMessage::Nothing
}
