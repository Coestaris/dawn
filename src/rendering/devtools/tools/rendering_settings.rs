use crate::rendering::config::{BoundingBoxMode, OutputMode, RenderingConfig};

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

impl OutputMode {
    pub fn items() -> [&'static str; 8] {
        [
            OutputMode::Default.as_str(),
            OutputMode::AlbedoOnly.as_str(),
            OutputMode::MetallicOnly.as_str(),
            OutputMode::NormalOnly.as_str(),
            OutputMode::RoughnessOnly.as_str(),
            OutputMode::OcclusionOnly.as_str(),
            OutputMode::DepthOnly.as_str(),
            OutputMode::Position.as_str(),
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            OutputMode::Default => "Default",
            OutputMode::AlbedoOnly => "Albedo Only",
            OutputMode::MetallicOnly => "Metallic Only",
            OutputMode::NormalOnly => "Normal Only",
            OutputMode::RoughnessOnly => "Roughness Only",
            OutputMode::OcclusionOnly => "Occlusion Only",
            OutputMode::DepthOnly => "Depth Only",
            OutputMode::Position => "Position",
        }
    }
}

impl From<usize> for OutputMode {
    fn from(value: usize) -> Self {
        match value {
            0 => OutputMode::Default,
            1 => OutputMode::AlbedoOnly,
            2 => OutputMode::MetallicOnly,
            3 => OutputMode::NormalOnly,
            4 => OutputMode::RoughnessOnly,
            5 => OutputMode::OcclusionOnly,
            6 => OutputMode::DepthOnly,
            7 => OutputMode::Position,

            _ => {
                panic!("Unknown output texture index {}", value);
            }
        }
    }
}

impl RenderingConfig {
    fn set_wireframe(&mut self, enabled: bool) {
        self.0.borrow_mut().wireframe = enabled;
    }

    fn set_fxaa_enabled(&mut self, enabled: bool) {
        self.0.borrow_mut().fxaa_enabled = enabled;
    }

    fn set_output_mode(&mut self, mode: OutputMode) {
        self.0.borrow_mut().output_mode = mode;
    }

    fn set_bounding_box_mode(&mut self, mode: BoundingBoxMode) {
        self.0.borrow_mut().bounding_box_mode = mode;
    }

    fn set_show_gizmos(&mut self, enabled: bool) {
        self.0.borrow_mut().show_gizmos = enabled;
    }
}

pub fn tool_rendering_settings(ui: &egui::Context, config: &mut RenderingConfig) {
    egui::Window::new("Rendering settings")
        .resizable(true)
        .fade_in(true)
        .fade_out(true)
        .collapsible(true)
        .show(ui, |ui| {
            ui.checkbox(&mut config.0.borrow_mut().wireframe, "Wireframe");

            ui.checkbox(&mut config.0.borrow_mut().fxaa_enabled, "FXAA");

            let mut output_mode = config.0.borrow().output_mode as usize;
            egui::ComboBox::from_label("Output Mode")
                .selected_text(OutputMode::from(output_mode).as_str())
                .show_ui(ui, |ui| {
                    for (i, item) in OutputMode::items().iter().enumerate() {
                        ui.selectable_value(&mut output_mode, i, *item);
                    }
                });
            config.set_output_mode(OutputMode::from(output_mode));

            let mut bbox_mode = config.0.borrow().bounding_box_mode as usize;
            egui::ComboBox::from_label("Bounding Box Mode")
                .selected_text(BoundingBoxMode::from(bbox_mode).as_str())
                .show_ui(ui, |ui| {
                    for (i, item) in BoundingBoxMode::items().iter().enumerate() {
                        ui.selectable_value(&mut bbox_mode, i, *item);
                    }
                });
            config.set_bounding_box_mode(BoundingBoxMode::from(bbox_mode));

            ui.checkbox(&mut config.0.borrow_mut().show_gizmos, "Show Gizmos");
        });
}
