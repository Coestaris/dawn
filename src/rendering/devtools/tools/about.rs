use crate::logging::dawn_build_info;
use build_info::VersionControl;
use egui::RichText;
use egui::text::LayoutJob;
use log::info;

pub fn warn_if_debug_build(ui: &mut egui::Ui) {
    if cfg!(debug_assertions) {
        ui.label(
            RichText::new("⚠ Debug build ⚠")
                .color(ui.visuals().warn_fg_color),
        )
            .on_hover_text("Project was compiled with debug assertions enabled. This may lead to lower performance.");
    }
}

pub fn add_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value));
        });
    });
}

pub fn tool_about(ui: &egui::Context) {
    egui::Window::new("About Dawn")
        .show(ui, |ui| {
        ui.vertical(|ui| {
            let bi = dawn_build_info();
            ui.heading(RichText::new("Dawn"));
            warn_if_debug_build(ui);
            ui.separator();

             add_row(ui, "Version:", &format!("{}", bi.crate_info.version));
             add_row(ui, "Features: ", &format!("{}", bi.crate_info.enabled_features.join(", ")));
             add_row(ui, "Build Profile:", &format!("{}", bi.profile));
             add_row(ui, "Optimization level:", &format!("{}", bi.optimization_level));
             add_row(ui, "Target:", &format!("{}", bi.target));
             add_row(ui, "Compiler:", &format!("{}", bi.compiler));
            if let Some(VersionControl::Git(git)) = &bi.version_control {
                ui.separator();
                add_row(ui, "Commit:", &git.commit_id);
                add_row(ui, "Commit time:", format!("{}", git.commit_timestamp).as_str());
                add_row(ui, "Is dirty:", &format!("{}", git.dirty));
                add_row(ui, "Branch:", &git.branch.as_deref().unwrap_or("N/A"));
                add_row(ui, "Tags:", &git.tags.join(", "));
            }
        });
    });
}
