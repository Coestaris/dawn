use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::primitive::quad::Quad;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::framebuffer::{
    BlitFramebufferFilter, BlitFramebufferMask, Framebuffer,
};
use dawn_graphics::gl::raii::shader_program::{Program, UniformBlockLocation, UniformLocation};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::{UVec2, Vec2};
use glow::HasContext;
use std::rc::Rc;

struct ShaderContainer {
    shader: TypedAsset<Program>,
    ubo_camera_location: UniformBlockLocation,
    texture_location: UniformLocation,
    size_location: UniformLocation,
    position_location: UniformLocation,
}

pub(crate) struct GizmosPass {
    gl: &'static glow::Context,
    id: RenderPassTargetId,
    shader: Option<ShaderContainer>,

    viewport_size: UVec2,
    quad: Quad,
    light_texture: Option<TypedAsset<Texture>>,

    gbuffer: Rc<GBuffer>,
    config: RenderingConfig,
}

impl GizmosPass {
    pub fn new(
        gl: &'static glow::Context,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        config: RenderingConfig,
    ) -> Self {
        GizmosPass {
            gl,
            id,
            shader: None,
            viewport_size: Default::default(),
            quad: Quad::new(gl),
            light_texture: None,
            gbuffer,
            config,
        }
    }
}

impl RenderPass<RenderingEvent> for GizmosPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_gizmos_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut GizmosPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_gizmos_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.shader = None;
                self.light_texture = None;
            }
            RenderingEvent::ViewportResized(size) => {
                self.viewport_size = size;
            }
            RenderingEvent::UpdateShader(shader) => {
                let clone = shader.clone();
                let casted = shader.cast();
                self.shader = Some(ShaderContainer {
                    shader: clone,
                    ubo_camera_location: casted.get_uniform_block_location("ubo_camera").unwrap(),
                    texture_location: casted.get_uniform_location("in_sprite").unwrap(),
                    size_location: casted.get_uniform_location("in_size").unwrap(),
                    position_location: casted.get_uniform_location("in_position").unwrap(),
                });

                if let Some(shader) = self.shader.as_mut() {
                    let program = shader.shader.cast();
                    Program::bind(self.gl, &program);
                    program.set_uniform_block_binding(
                        shader.ubo_camera_location,
                        CAMERA_UBO_BINDING as u32,
                    );
                    program.set_uniform(shader.texture_location, 0);
                    program.set_uniform(shader.size_location, Vec2::new(0.7, 0.7));
                    Program::unbind(self.gl);
                }
            }
            RenderingEvent::SetLightTexture(texture) => {
                self.light_texture = Some(texture);
            }

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "GizmosPass"
    }

    fn begin(
        &mut self,
        _backend: &RendererBackend<RenderingEvent>,
        frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }
        if self.light_texture.is_none() {
            return RenderResult::default();
        }
        if !self.config.get_show_gizmos() {
            return RenderResult::default();
        }

        Framebuffer::blit_to_default(
            self.gl,
            &self.gbuffer.fbo,
            self.viewport_size,
            BlitFramebufferMask::Depth,
            BlitFramebufferFilter::Nearest,
        );

        unsafe {
            // Enable blending for transparency
            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.gl.enable(glow::DEPTH_TEST);
        }

        let shader = self.shader.as_ref().unwrap();
        let program = shader.shader.cast();
        Program::bind(self.gl, &program);

        let light_texture = self.light_texture.as_ref().unwrap().cast();
        Texture::bind(self.gl, TextureBind::Texture2D, light_texture, 0);

        let mut result = RenderResult::default();

        for point_light in frame.point_lights.iter() {
            let position = point_light.position;
            program.set_uniform(shader.position_location, position);
            result += self.quad.draw();
        }

        result
    }

    fn end(&mut self, _backend: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        if !self.config.get_show_gizmos() {
            return RenderResult::default();
        }

        Program::unbind(self.gl);

        RenderResult::default()
    }
}
