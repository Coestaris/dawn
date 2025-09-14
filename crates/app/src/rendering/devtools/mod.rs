mod compositor;
mod tools;

use crate::devtools::DevtoolsRendererConnection;
use crate::rendering::config::RenderingConfig;
use crate::rendering::devtools::compositor::Compositor;
use crate::rendering::event::RenderingEvent;
use dawn_graphics::renderer::RendererBackend;
use std::sync::Arc;
use build_info::BuildInfo;
use winit::window::Window;

pub struct DevToolsGUI {
    egui_winit: Option<egui_winit::State>,
    egui_glow: Option<egui_glow::Painter>,
    compositor: Compositor,
}

impl DevToolsGUI {
    pub(crate) fn new(config: RenderingConfig, connection: DevtoolsRendererConnection, bi: BuildInfo) -> Self {
        DevToolsGUI {
            egui_winit: None,
            egui_glow: None,
            compositor: Compositor::new(connection, config, bi),
        }
    }

    pub fn attach_to_window(&mut self, w: &Window, r: &RendererBackend<RenderingEvent>) {
        self.compositor.update_gl_info(r.info.clone());

        if self.egui_glow.is_none() {
            let painter =
                egui_glow::Painter::new(Arc::new(r.new_context().unwrap()), "", None, true)
                    .expect("Failed to initialize egui_glow painter");
            self.egui_glow = Some(painter);
        }

        if self.egui_winit.is_none() {
            let egui = egui::Context::default();
            let mut visuals = egui::Visuals::dark();
            visuals.window_fill = egui::Color32::from_rgba_unmultiplied(
                visuals.window_fill.r(),
                visuals.window_fill.g(),
                visuals.window_fill.b(),
                200,
            );

            egui.set_visuals(visuals);
            egui.set_zoom_factor(1.1);

            let id = egui.viewport_id();
            let state = egui_winit::State::new(egui, id, w, None, None, None);
            self.egui_winit = Some(state);
        }
    }

    pub fn on_window_event(&mut self, window: &Window, event: &winit::event::WindowEvent) {
        if let Some(egui_winit) = &mut self.egui_winit {
            let _ = egui_winit.on_window_event(&window, event);
        }
    }

    pub fn before_frame(&mut self, _window: &Window, _backend: &RendererBackend<RenderingEvent>) {
        self.compositor.before_frame();
    }

    pub fn after_render(&mut self, window: &Window, _backend: &RendererBackend<RenderingEvent>) {
        // Render UI here if needed
        if let (Some(egui_winit), Some(egui_glow)) = (&mut self.egui_winit, &mut self.egui_glow) {
            let raw_input = egui_winit.take_egui_input(&window);
            let full_output = egui_winit.egui_ctx().run(raw_input, |ctx| {
                self.compositor.run(ctx);
            });
            egui_winit.handle_platform_output(&window, full_output.platform_output);

            let pixels_per_point = egui_winit.egui_ctx().pixels_per_point();
            let clipped_meshes = egui_winit
                .egui_ctx()
                .tessellate(full_output.shapes, pixels_per_point);
            let screen_size_in_pixels = window.inner_size();

            egui_glow.paint_and_update_textures(
                screen_size_in_pixels.into(),
                pixels_per_point,
                &clipped_meshes,
                &full_output.textures_delta,
            );
        }
    }
}

impl Drop for DevToolsGUI {
    fn drop(&mut self) {
        if let Some(mut painter) = self.egui_glow.take() {
            painter.destroy();
        }
    }
}
