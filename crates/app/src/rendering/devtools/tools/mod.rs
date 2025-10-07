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

pub fn row3_duration(
    row: &mut egui_extras::TableRow,
    label: &str,
    sample: MonitorSample<web_time::Duration>,
) {
    row.col(|ui| {
        ui.strong(label);
    });
    row.col(|ui| {
        ui.label(format!("{:.1?}", sample.min()));
    });
    row.col(|ui| {
        ui.label(format!("{:.1?}", sample.average()));
    });
    row.col(|ui| {
        ui.label(format!("{:.1?}", sample.max()));
    });
}

pub fn row2_duration(
    row: &mut egui_extras::TableRow,
    label: &str,
    sample1: MonitorSample<web_time::Duration>,
    sample2: MonitorSample<web_time::Duration>,
) {
    row.col(|ui| {
        ui.strong(label);
    });
    row.col(|ui| {
        ui.label(format!("{:.1?}", sample1.average()));
    });
    row.col(|ui| {
        ui.label(format!("{:.1?}", sample2.average()));
    });
}

pub fn row3_f32(row: &mut egui_extras::TableRow, label: &str, sample: MonitorSample<f32>) {
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

pub fn row3_f32_s(row: &mut egui_extras::TableRow, label: &str, sample: MonitorSample<f32>) {
    row.col(|ui| {
        ui.strong(label);
    });
    row.col(|ui| {
        ui.label(format!("{:.0}", sample.min()));
    });
    row.col(|ui| {
        ui.label(format!("{:.0}", sample.average()));
    });
    row.col(|ui| {
        ui.label(format!("{:.0}", sample.max()));
    });
}

pub fn mul_sample(sample: MonitorSample<f32>, factor: f32) -> MonitorSample<f32> {
    MonitorSample::new(
        sample.min() * factor,
        sample.average() * factor,
        sample.max() * factor,
    )
}
