use crate::devtools::SunlightControl;
use crate::rendering::config::{
    generate_ssao_kernel, BoundingBoxMode, OutputMode, RenderingConfig,
};
use egui::Widget;

pub enum ToolRenderingSettingsMessage {
    Nothing,
    ControlSunlight,
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

impl OutputMode {
    pub fn items() -> [&'static str; 9] {
        [
            OutputMode::Default.as_str(),
            OutputMode::AlbedoOnly.as_str(),
            OutputMode::MetallicOnly.as_str(),
            OutputMode::NormalOnly.as_str(),
            OutputMode::RoughnessOnly.as_str(),
            OutputMode::OcclusionOnly.as_str(),
            OutputMode::DepthOnly.as_str(),
            OutputMode::Position.as_str(),
            OutputMode::SSAOOnly.as_str(),
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
            OutputMode::SSAOOnly => "SSAO Only",
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
            8 => OutputMode::SSAOOnly,

            _ => {
                panic!("Unknown output texture index {}", value);
            }
        }
    }
}

pub fn tool_rendering_settings(
    ui: &egui::Context,
    config: &mut RenderingConfig,
    sunlight_control: &mut SunlightControl,
) -> ToolRenderingSettingsMessage {
    let mut result = ToolRenderingSettingsMessage::Nothing;
    let mut config = config.0.borrow_mut();

    egui::Window::new("🔧 Rendering settings")
        .resizable(true)
        .fade_in(true)
        .fade_out(true)
        .collapsible(true)
        .show(ui, |ui| {
            let output_mode = &mut config.general.output_mode;
            let mut output_mode_code = *output_mode as usize;
            egui::ComboBox::from_label("Output Mode")
                .selected_text(OutputMode::from(output_mode_code).as_str())
                .show_ui(ui, |ui| {
                    for (i, item) in OutputMode::items().iter().enumerate() {
                        ui.selectable_value(&mut output_mode_code, i, *item);
                    }
                });
            *output_mode = OutputMode::from(output_mode_code);

            let bbox_mode = &mut config.general.bounding_box_mode;
            let mut bbox_mode_code = *bbox_mode as usize;
            egui::ComboBox::from_label("Bounding Box Mode")
                .selected_text(BoundingBoxMode::from(bbox_mode_code).as_str())
                .show_ui(ui, |ui| {
                    for (i, item) in BoundingBoxMode::items().iter().enumerate() {
                        ui.selectable_value(&mut bbox_mode_code, i, *item);
                    }
                });
            *bbox_mode = BoundingBoxMode::from(bbox_mode_code);

            ui.checkbox(&mut config.general.show_gizmos, "Show Gizmos");
            ui.checkbox(&mut config.general.wireframe, "Wireframe");

            ui.checkbox(&mut config.general.fxaa_enabled, "FXAA");
            ui.checkbox(&mut config.general.ssao_enabled, "SSAO");

            ui.checkbox(&mut config.lighting.force_no_tangents, "Force No Tangents");

            ui.collapsing("Lighting Settings", |ui| {
                egui::Slider::new(&mut config.lighting.sky_color.x, 0.0..=1.0)
                    .text("Sky Color R")
                    .ui(ui);
                egui::Slider::new(&mut config.lighting.sky_color.y, 0.0..=1.0)
                    .text("Sky Color G")
                    .ui(ui);
                egui::Slider::new(&mut config.lighting.sky_color.z, 0.0..=1.0)
                    .text("Sky Color B")
                    .ui(ui);
                egui::Slider::new(&mut config.lighting.ground_color.x, 0.0..=1.0)
                    .text("Ground Color R")
                    .ui(ui);
                egui::Slider::new(&mut config.lighting.ground_color.y, 0.0..=1.0)
                    .text("Ground Color G")
                    .ui(ui);
                egui::Slider::new(&mut config.lighting.ground_color.z, 0.0..=1.0)
                    .text("Ground Color B")
                    .ui(ui);
                egui::Slider::new(&mut config.lighting.diffuse_scale, 0.0..=10.0)
                    .text("Diffuse Scale")
                    .ui(ui);
                egui::Slider::new(&mut config.lighting.specular_scale, 0.0..=10.0)
                    .text("Specular Scale")
                    .ui(ui);
            });

            ui.collapsing("Sunlight Settings", |ui| {
                let mut changed = false;
                changed |= egui::Slider::new(&mut sunlight_control.intensity, 0.0..=10.0)
                    .text("Intensity")
                    .ui(ui)
                    .changed();
                changed |= egui::Slider::new(&mut sunlight_control.ambient, 0.0..=1.0)
                    .text("Ambient")
                    .ui(ui)
                    .changed();
                changed |= egui::Slider::new(&mut sunlight_control.color.x, 0.0..=1.0)
                    .text("Color R")
                    .ui(ui)
                    .changed();
                changed |= egui::Slider::new(&mut sunlight_control.color.y, 0.0..=1.0)
                    .text("Color G")
                    .ui(ui)
                    .changed();
                changed |= egui::Slider::new(&mut sunlight_control.color.z, 0.0..=1.0)
                    .text("Color B")
                    .ui(ui)
                    .changed();
                changed |= egui::Slider::new(&mut sunlight_control.direction.x, -1.0..=1.0)
                    .text("Direction X")
                    .ui(ui)
                    .changed();
                changed |= egui::Slider::new(&mut sunlight_control.direction.y, -1.0..=1.0)
                    .text("Direction Y")
                    .ui(ui)
                    .changed();
                changed |= egui::Slider::new(&mut sunlight_control.direction.z, -1.0..=1.0)
                    .text("Direction Z")
                    .ui(ui)
                    .changed();

                if changed {
                    result = ToolRenderingSettingsMessage::ControlSunlight;
                }
            });

            ui.collapsing("SSAO Raw", |ui| {
                if egui::Slider::new(&mut config.ssao_raw.kernel_size, 1..=64)
                    .text("Kernel Size")
                    .ui(ui)
                    .changed()
                {
                    config.ssao_raw.kernel =
                        generate_ssao_kernel(config.ssao_raw.kernel_size as usize);
                }
                egui::Slider::new(&mut config.ssao_raw.radius, 0.01..=5.0)
                    .text("Radius")
                    .ui(ui);
                egui::Slider::new(&mut config.ssao_raw.bias, 0.0..=0.5)
                    .text("Bias")
                    .ui(ui);
                egui::Slider::new(&mut config.ssao_raw.intensity, 0.0..=10.0)
                    .text("Intensity")
                    .ui(ui);
                egui::Slider::new(&mut config.ssao_raw.power, 0.1..=5.0)
                    .text("Power")
                    .ui(ui);
            });

            ui.collapsing("SSAO Blur", |ui| {
                egui::Slider::new(&mut config.ssao_blur.radius, 1..=8)
                    .text("Blur radius")
                    .ui(ui);
                egui::Slider::new(&mut config.ssao_blur.sigma_spatial, 0.1..=40.0)
                    .text("Sigma Spatial")
                    .ui(ui);
                egui::Slider::new(&mut config.ssao_blur.sigma_normal, 8.0..=128.0)
                    .text("Sigma Normal")
                    .ui(ui);
            });
        });

    result
}
