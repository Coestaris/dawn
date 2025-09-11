use crate::rendering::devtools::tools::{mul_sample, row_duration, row_f32};
use crate::world::devtools::WorldStatistics;
use dawn_ecs::world::WorldLoopMonitorEvent;
use egui_extras::{Column, TableBuilder};

pub fn tool_world_stat(
    ui: &egui::Context,
    monitor: &WorldLoopMonitorEvent,
    stat: &WorldStatistics,
) {
    egui::Window::new("World Statistics").show(ui, |ui| {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto().resizable(true).at_least(200.0))
            .column(Column::remainder())
            .column(Column::remainder())
            .column(Column::remainder())
            .header(text_height, |mut header| {
                header.col(|ui| {
                    ui.strong("Statistic");
                });
                header.col(|ui| {
                    ui.strong("Min");
                });
                header.col(|ui| {
                    ui.strong("Average");
                });
                header.col(|ui| {
                    ui.strong("Max");
                });
            })
            .body(|mut body| {
                body.row(text_height, |mut row| {
                    row_f32(&mut row, "Update Time (ms)", monitor.tps);
                });
                body.row(text_height, |mut row| {
                    row_f32(&mut row, "Load (percent)", mul_sample(monitor.load, 100.0));
                });
                body.row(text_height, |mut row| {
                    row_duration(&mut row, "Cycle time", monitor.cycle_time);
                });
            });

        ui.separator();

        ui.horizontal(|ui| {
            ui.strong("Entities: ");
            ui.label(format!("{}", stat.entities));
        });
        ui.horizontal(|ui| {
            ui.strong("Drawables: ");
            ui.label(format!("{}", stat.drawables));
        });
        ui.horizontal(|ui| {
            ui.strong("Point Lights: ");
            ui.label(format!("{}", stat.point_lights));
        });
        ui.horizontal(|ui| {
            ui.strong("Spot Lights: ");
            ui.label(format!("{}", stat.spot_lights));
        });
        ui.horizontal(|ui| {
            ui.strong("Sun Lights: ");
            ui.label(format!("{}", stat.sun_lights));
        });
        ui.horizontal(|ui| {
            ui.strong("Area Lights: ");
            ui.label(format!("{}", stat.area_lights));
        });
    });
}
