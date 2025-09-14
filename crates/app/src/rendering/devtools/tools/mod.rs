use dawn_util::profile::MonitorSample;

pub mod about;
pub mod assets_info;
pub mod controls;
pub mod rendering_settings;
pub mod rendering_stat;
pub mod world_stat;

pub fn row_height() -> f32 {
    egui::TextStyle::Body
        .resolve(&egui::Style::default())
        .size
        .max(20.0)
}

pub fn row_duration(
    row: &mut egui_extras::TableRow,
    label: &str,
    sample: MonitorSample<std::time::Duration>,
) {
    row.col(|ui| {
        ui.strong(label);
    });
    row.col(|ui| {
        ui.label(format!("{:.2?}", sample.min()));
    });
    row.col(|ui| {
        ui.label(format!("{:.2?}", sample.average()));
    });
    row.col(|ui| {
        ui.label(format!("{:.2?}", sample.max()));
    });
}

pub fn row_f32(row: &mut egui_extras::TableRow, label: &str, sample: MonitorSample<f32>) {
    row.col(|ui| {
        ui.strong(label);
    });
    row.col(|ui| {
        ui.label(format!("{:.2}", sample.min()));
    });
    row.col(|ui| {
        ui.label(format!("{:.2}", sample.average()));
    });
    row.col(|ui| {
        ui.label(format!("{:.2}", sample.max()));
    });
}

pub fn row_f32_s(row: &mut egui_extras::TableRow, label: &str, sample: MonitorSample<f32>) {
    row.col(|ui| {
        ui.strong(label);
    });
    row.col(|ui| {
        ui.label(format!("{:.2e}", sample.min()));
    });
    row.col(|ui| {
        ui.label(format!("{:.2e}", sample.average()));
    });
    row.col(|ui| {
        ui.label(format!("{:.2e}", sample.max()));
    });
}

pub fn mul_sample(sample: MonitorSample<f32>, factor: f32) -> MonitorSample<f32> {
    MonitorSample::new(
        sample.min() * factor,
        sample.average() * factor,
        sample.max() * factor,
    )
}
