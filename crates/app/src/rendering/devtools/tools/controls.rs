pub fn tool_controls(ui: &egui::Context) {
    egui::Window::new("🔦 Controls")
        .resizable(true)
        .fade_in(true)
        .fade_out(true)
        .collapsible(true)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label("Use the following controls to navigate the 3D scene:");
                ui.horizontal_wrapped(|ui| {
                    ui.strong("W/A/S/D");
                    ui.label("to move forward/left/backward/right");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Space");
                    ui.label("to move up");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Left Shift");
                    ui.label("to move down");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Right Mouse Drag");
                    ui.label("to look around");
                });

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label("Use");
                    ui.strong("arrow keys");
                    ui.label("and");
                    ui.strong("Page Up/Page Down");
                    ui.label("to move the Point Light(s) in the scene.");
                });

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label("Press");
                    ui.strong("F5");
                    ui.label("to reload all assets.");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Press");
                    ui.strong("F11");
                    ui.label("to toggle fullscreen mode.");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Press");
                    ui.strong("ESC");
                    ui.label("to exit the application.");
                });
            });
        });
}
