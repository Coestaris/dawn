use dawn_graphics::renderer::RendererMonitorEvent;
use dawn_util::profile::MonitorSample;
use egui_extras::{Column, TableBuilder};
use crate::rendering::devtools::tools::{mul_sample, row_duration, row_f32, row_f32_s};

pub fn tool_rendering_stat(ui: &egui::Context, stat: &RendererMonitorEvent) {
    egui::Window::new("Rendering Statistics")
        .resizable(true)
        .fade_in(true)
        .fade_out(true)
        .collapsible(true)
        .show(ui, |ui| {
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
                    ui.strong("");
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
                    row_f32(&mut row, "FPS", stat.fps);
                });
                body.row(text_height, |mut row| {
                    row_f32(&mut row, "Load (percent)", mul_sample(stat.load, 100.0));
                });
                body.row(text_height, |mut row| {
                    row_duration(&mut row, "Render Time", stat.render);
                });
                body.row(text_height, |mut row| {
                    row.set_overline(true);
                    row_duration(&mut row, "View Time", stat.view);
                });
                body.row(text_height, |mut row| {
                    row_duration(&mut row, "Events Time", stat.events);
                });
                body.row(text_height, |mut row| {
                    row_f32_s(&mut row, "Primitives (per sec)", stat.drawn_primitives);
                });
                body.row(text_height, |mut row| {
                    row_f32_s(&mut row, "Draw calls (per sec)", stat.draw_calls);
                });
                for (i, (pass, time)) in stat.passes.iter().enumerate() {
                    body.row(text_height, |mut row| {
                        if i == 0 {
                            row.set_overline(true);
                        }

                        row_duration(&mut row, pass, *time);
                    });
                }
            });
    });
}
