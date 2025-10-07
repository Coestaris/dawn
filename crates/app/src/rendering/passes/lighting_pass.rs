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
use winit::window::Window;

const DEPTH_INDEX: i32 = 0;
const ALBEDO_INDEX: i32 = 1;
const ORM_INDEX: i32 = 2;
const NORMAL_INDEX: i32 = 3;
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
                program.set_uniform(&shader.depth, DEPTH_INDEX);
                program.set_uniform(&shader.albedo, ALBEDO_INDEX);
                program.set_uniform(&shader.orm, ORM_INDEX);
                program.set_uniform(&shader.normal, NORMAL_INDEX);
                program.set_uniform(&shader.packed_lights, PACKED_LIGHTS_INDEX);
                program.set_uniform(&shader.ssao, SSAO_INDEX);
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
        _: &Window,
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
        program.set_uniform(&shader.packed_lights_header, header.as_uvec4());
        self.gbuffer.depth.bind2d(DEPTH_INDEX);
        self.gbuffer.albedo.bind2d(ALBEDO_INDEX);
        self.gbuffer.orm.bind2d(ORM_INDEX);
        self.gbuffer.normal.bind2d(NORMAL_INDEX);
        self.packed_lights.bind(PACKED_LIGHTS_INDEX);
        self.ssao_blurred.texture.bind2d(SSAO_INDEX);

        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &Window, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Framebuffer::unbind(&self.gl);
        Program::unbind(&self.gl);
        Texture::unbind(&self.gl, TextureBind::Texture2D, DEPTH_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, ALBEDO_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, ORM_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, NORMAL_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, PACKED_LIGHTS_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, SSAO_INDEX as u32);
        RenderResult::default()
    }
}
