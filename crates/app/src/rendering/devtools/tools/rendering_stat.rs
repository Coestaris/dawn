use crate::rendering::devtools::tools::{
    mul_sample, row2_duration, row3_duration, row3_f32, row3_f32_s, row_height,
};
use dawn_graphics::renderer::RendererMonitorEvent;
use egui_extras::{Column, TableBuilder};

pub fn tool_rendering_stat(ui: &egui::Context, stat: &RendererMonitorEvent) {
    egui::Window::new("💻 Rendering Statistics")
        .resizable(true)
        .fade_in(true)
        .fade_out(true)
        .collapsible(true)
        .show(ui, |ui| {
            let text_height = row_height();

            ui.collapsing("Overall Statistics", |ui| {
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
                            row3_f32(&mut row, "FPS", stat.fps);
                        });
                        body.row(text_height, |mut row| {
                            row3_f32(&mut row, "Load (percent)", mul_sample(stat.load, 100.0));
                        });
                        body.row(text_height, |mut row| {
                            row3_duration(&mut row, "Render Time", stat.render);
                        });
                        body.row(text_height, |mut row| {
                            row.set_overline(true);
                            row3_duration(&mut row, "View Time", stat.view);
                        });
                        body.row(text_height, |mut row| {
                            row3_duration(&mut row, "Events Time", stat.events);
                        });
                        body.row(text_height, |mut row| {
                            row3_f32_s(
                                &mut row,
                                "Primitives (per frame)",
                                mul_sample(stat.drawn_primitives, 1.0 / stat.fps.average()),
                            );
                        });
                        body.row(text_height, |mut row| {
                            row3_f32_s(
                                &mut row,
                                "Draw calls (per frame)",
                                mul_sample(stat.draw_calls, 1.0 / stat.fps.average()),
                            );
                        });
                    });
            });

            ui.collapsing("Per Render Pass Statistics", |ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::auto().resizable(true).at_least(200.0))
                    .column(Column::remainder())
                    .column(Column::remainder())
                    .header(text_height, |mut header| {
                        header.col(|ui| {
                            ui.strong("Render Pass");
                        });
                        header.col(|ui| {
                            ui.strong("CPU Time");
                        });
                        header.col(|ui| {
                            ui.strong("GPU Time");
                        });
                    })
                    .body(|mut body| {
                        for (i, (pass, (cpu, gpu))) in stat
                            .pass_names
                            .iter()
                            .zip(stat.pass_cpu_times.iter().zip(stat.pass_gpu_times.iter()))
                            .enumerate()
                        {
                            body.row(text_height, |mut row| {
                                if i == 0 {
                                    row.set_overline(true);
                                }
                                row2_duration(&mut row, pass, *cpu, *gpu)
                            });
                        }
                    });
            });
        });
}
