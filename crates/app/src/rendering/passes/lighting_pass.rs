use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::fbo::obuffer::LightningTarget;
use crate::rendering::fbo::ssao::SSAOTarget;
use crate::rendering::primitive::quad::Quad2D;
use crate::rendering::shaders::lighting::LightingShader;
use crate::rendering::ubo::packed_light::{LightsHeaderPayload, PackedLights};
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glow::HasContext;
use std::rc::Rc;
use std::sync::Arc;

const POSITION_INDEX: i32 = 0;
const ALBEDO_METALLIC_INDEX: i32 = 1;
const NORMAL_INDEX: i32 = 2;
const PBR_INDEX: i32 = 3;
const PACKED_LIGHTS_INDEX: i32 = 4;
const SSAO_INDEX: i32 = 5;

pub(crate) struct LightingPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<LightingShader>,
    quad: Quad2D,
    view: glam::Mat4,
    packed_lights: PackedLights,
    ssao_blurred: Rc<SSAOTarget>,
    gbuffer: Rc<GBuffer>,
    target: Rc<LightningTarget>,
}

impl LightingPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        ssao_blurred: Rc<SSAOTarget>,
        target: Rc<LightningTarget>,
        config: RenderingConfig,
    ) -> Self {
        LightingPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            quad: Quad2D::new(gl.clone()),
            view: glam::Mat4::IDENTITY,
            packed_lights: PackedLights::new(gl).unwrap(),
            ssao_blurred,
            gbuffer,
            target,
        }
    }
}

impl RenderPass<RenderingEvent> for LightingPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut LightingPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.shader = None;
            }
            RenderingEvent::ViewUpdated(view) => {
                self.view = view;
            }
            RenderingEvent::UpdateShader(_, shader) => {
                self.shader = Some(LightingShader::new(shader.clone()).unwrap());

                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform(&shader.position_texture, POSITION_INDEX);
                program.set_uniform(&shader.albedo_metallic_texture, ALBEDO_METALLIC_INDEX);
                program.set_uniform(&shader.normal_texture, NORMAL_INDEX);
                program.set_uniform(&shader.pbr_texture, PBR_INDEX);
                program.set_uniform(&shader.packed_lights_location, PACKED_LIGHTS_INDEX);
                program.set_uniform(&shader.ssao_texture, SSAO_INDEX);
                Program::unbind(&self.gl);
            }
            RenderingEvent::ViewportResized(size) => {
                self.gbuffer.resize(size);
                self.target.resize(size);
            }

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "LightingPass"
    }

    #[inline(always)]
    fn begin(
        &mut self,
        _: &RendererBackend<RenderingEvent>,
        frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
        }

        self.packed_lights.clear();
        let mut lights_count = 0;
        for light in frame.point_lights.iter() {
            self.packed_lights.push_point_light(light, &self.view);
            lights_count += 1;
        }
        for light in frame.sun_lights.iter() {
            self.packed_lights.push_sun_light(light, &self.view);
            lights_count += 1;
        }
        self.packed_lights.upload();
        let header = LightsHeaderPayload::new(lights_count as u32);

        Framebuffer::bind(&self.gl, &self.target.fbo);

        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, program);
        #[cfg(feature = "devtools")]
        {
            program.set_uniform(
                &shader.devtools.sky_color_location,
                self.config.get_sky_color(),
            );
            program.set_uniform(
                &shader.devtools.ssao_enabled,
                self.config.get_is_ssao_enabled() as i32,
            );
            program.set_uniform(
                &shader.devtools.ground_color_location,
                self.config.get_ground_color(),
            );
            program.set_uniform(
                &shader.devtools.diffuse_scale_location,
                self.config.get_diffuse_scale(),
            );
            program.set_uniform(
                &shader.devtools.specular_scale_location,
                self.config.get_specular_scale(),
            );
            program.set_uniform(
                &shader.devtools.debug_mode,
                self.config.get_output_mode() as i32,
            );
        }
        program.set_uniform(&shader.packed_lights_header_location, header.as_uvec4());
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.position.texture,
            POSITION_INDEX as u32,
        );
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.albedo_metallic.texture,
            ALBEDO_METALLIC_INDEX as u32,
        );
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.normal.texture,
            NORMAL_INDEX as u32,
        );
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.pbr.texture,
            PBR_INDEX as u32,
        );
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.packed_lights.texture,
            PACKED_LIGHTS_INDEX as u32,
        );
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.ssao_blurred.texture.texture,
            SSAO_INDEX as u32,
        );

        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Framebuffer::unbind(&self.gl);
        Program::unbind(&self.gl);
        Texture::unbind(&self.gl, TextureBind::Texture2D, POSITION_INDEX as u32);
        Texture::unbind(
            &self.gl,
            TextureBind::Texture2D,
            ALBEDO_METALLIC_INDEX as u32,
        );
        Texture::unbind(&self.gl, TextureBind::Texture2D, NORMAL_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, PBR_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, PACKED_LIGHTS_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, SSAO_INDEX as u32);
        RenderResult::default()
    }
}
