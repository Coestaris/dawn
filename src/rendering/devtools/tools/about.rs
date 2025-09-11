use crate::logging::dawn_build_info;
use build_info::VersionControl;
use dawn_graphics::gl::probe::OpenGLInfo;
use egui::text::LayoutJob;
use egui::RichText;

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

pub fn tool_about(ui: &egui::Context, glinfo: Option<&OpenGLInfo>) {
    egui::Window::new("🌃 About")
        .resizable(true)
        .fade_in(true)
        .fade_out(true)
        .scroll(true)
        .collapsible(true)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                let bi = dawn_build_info();
                ui.heading(RichText::new("Dawn"));
                warn_if_debug_build(ui);
                ui.separator();

                add_row(ui, "Version:", &format!("{}", bi.crate_info.version));
                add_row(
                    ui,
                    "Features: ",
                    &format!("{}", bi.crate_info.enabled_features.join(", ")),
                );
                add_row(ui, "Build Profile:", &format!("{}", bi.profile));
                add_row(
                    ui,
                    "Optimization level:",
                    &format!("{}", bi.optimization_level),
                );
                add_row(ui, "Target:", &format!("{}", bi.target));
                add_row(ui, "Compiler:", &format!("{}", bi.compiler));
                if let Some(VersionControl::Git(git)) = &bi.version_control {
                    ui.separator();
                    add_row(ui, "Commit:", &git.commit_id);
                    add_row(
                        ui,
                        "Commit time:",
                        format!("{}", git.commit_timestamp).as_str(),
                    );
                    add_row(ui, "Is dirty:", &format!("{}", git.dirty));
                    add_row(ui, "Branch:", &git.branch.as_deref().unwrap_or("N/A"));
                    add_row(ui, "Tags:", &git.tags.join(", "));
                }
                if let Some(glinfo) = glinfo {
                    ui.separator();
                    add_row(
                        ui,
                        "OpenGL Version:",
                        format!("{}", glinfo.version).as_str(),
                    );
                    add_row(ui, "Vendor:", &glinfo.vendor);
                    add_row(ui, "Renderer:", &glinfo.renderer);
                    let s = if let Some(sl) = &glinfo.shading_language_version {
                        format!("{}", sl)
                    } else {
                        "N/A".to_string()
                    };
                    ui.collapsing("Other", |ui| {
                        add_row(
                            ui,
                            "Depth Bits:",
                            &glinfo
                                .depth_bits
                                .map_or("N/A".to_string(), |d| d.to_string()),
                        );
                        add_row(
                            ui,
                            "Stencil Bits:",
                            &glinfo
                                .stencil_bits
                                .map_or("N/A".to_string(), |s| s.to_string()),
                        );
                        add_row(ui, "Shading Language Version:", &s);
                        // Binary formats
                        let mut job = LayoutJob::default();
                        for bf in &glinfo.binary_formats {
                            job.append("• ", 0.0, egui::TextFormat::default());
                            job.append(
                                format!("{}", bf).as_str(),
                                0.0,
                                egui::TextFormat::default(),
                            );
                            job.append("\n", 0.0, egui::TextFormat::default());
                        }
                        ui.label(job);
                    });
                    ui.collapsing("Extensions", |ui| {
                        let mut job = LayoutJob::default();
                        for ext in &glinfo.extensions {
                            job.append("• ", 0.0, egui::TextFormat::default());
                            job.append(ext, 0.0, egui::TextFormat::default());
                            job.append("\n", 0.0, egui::TextFormat::default());
                        }
                        ui.label(job);
                    });
                    ui.collapsing("Texture Limits", |ui| {
                        add_row(
                            ui,
                            "Max Texture Size:",
                            &format!("{}", glinfo.limits.texture.max_texture_size),
                        );
                        add_row(
                            ui,
                            "Max Cube Map Texture Size:",
                            &format!("{}", glinfo.limits.texture.max_cube_map_texture_size),
                        );
                        add_row(
                            ui,
                            "Max Texture Image Units:",
                            &format!("{}", glinfo.limits.texture.max_texture_image_units),
                        );
                        add_row(
                            ui,
                            "Max Combined Texture Image Units:",
                            &format!("{}", glinfo.limits.texture.max_combined_texture_image_units),
                        );
                    });
                    ui.collapsing("Buffer Limits", |ui| {
                        add_row(
                            ui,
                            "Max Vertex Attribs:",
                            &format!("{}", glinfo.limits.buffer.max_vertex_attribs),
                        );
                        add_row(
                            ui,
                            "Max Vertex Uniform Vectors:",
                            &format!("{}", glinfo.limits.buffer.max_vertex_uniform_vectors),
                        );
                        add_row(
                            ui,
                            "Max Fragment Uniform Vectors:",
                            &format!("{}", glinfo.limits.buffer.max_fragment_uniform_vectors),
                        );
                        add_row(
                            ui,
                            "Max Varying Vectors:",
                            &format!("{}", glinfo.limits.buffer.max_varying_vectors),
                        );
                        add_row(
                            ui,
                            "Max Combined Uniform Blocks:",
                            &format!("{}", glinfo.limits.buffer.max_combined_uniform_blocks),
                        );
                        add_row(
                            ui,
                            "Max Uniform Buffer Bindings:",
                            &format!("{}", glinfo.limits.buffer.max_uniform_buffer_bindings),
                        );
                        add_row(
                            ui,
                            "Max Uniform Block Size (bytes):",
                            &format!("{}", glinfo.limits.buffer.max_uniform_block_size),
                        );
                        add_row(
                            ui,
                            "Uniform Buffer Offset Alignment (bytes):",
                            &format!("{}", glinfo.limits.buffer.uniform_buffer_offset_alignment),
                        );
                    });
                    ui.collapsing("Shader Limits", |ui| {
                        add_row(
                            ui,
                            "Max Vertex Shader Storage Blocks:",
                            &format!("{}", glinfo.limits.shader.max_vertex_shader_storage_blocks),
                        );
                        add_row(
                            ui,
                            "Max Fragment Shader Storage Blocks:",
                            &format!(
                                "{}",
                                glinfo.limits.shader.max_fragment_shader_storage_blocks
                            ),
                        );
                        add_row(
                            ui,
                            "Max Combined Shader Storage Blocks:",
                            &format!(
                                "{}",
                                glinfo.limits.shader.max_combined_shader_storage_blocks
                            ),
                        );
                    });
                    ui.collapsing("Framebuffer Limits", |ui| {
                        add_row(
                            ui,
                            "Max Color Attachments:",
                            &format!("{}", glinfo.limits.framebuffer.max_color_attachments),
                        );
                        add_row(
                            ui,
                            "Max Draw Buffers:",
                            &format!("{}", glinfo.limits.framebuffer.max_draw_buffers),
                        );
                    });
                }
            });
        });
}
