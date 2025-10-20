use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::fbo::lighting::LightingTarget;
use crate::rendering::fbo::ssao::SSAOHalfresTarget;
use crate::rendering::primitive::quad::Quad2D;
use crate::rendering::shaders::lighting::LightingShader;
use crate::rendering::ubo::packed_light::{LightInfo, LightsHeaderPayload, PackedLights};
use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::{Texture2D, TextureCube};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glow::HasContext;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;

const DEPTH_INDEX: i32 = 0;
const ALBEDO_INDEX: i32 = 1;
const ORM_INDEX: i32 = 2;
const NORMAL_INDEX: i32 = 3;
const PACKED_LIGHTS_INDEX: i32 = 4;
const HALFRES_SSAO_INDEX: i32 = 5;
const SKYBOX_INDEX: i32 = 6;

pub(crate) struct LightingPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,
    light_info: Rc<RefCell<LightInfo>>,

    shader: Option<LightingShader>,
    skybox: Option<TypedAsset<TextureCube>>,
    quad: Quad2D,
    view: glam::Mat4,
    halfres_ssao: Rc<SSAOHalfresTarget>,
    gbuffer: Rc<GBuffer>,
    target: Rc<LightingTarget>,
}

impl LightingPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        ssao_blurred: Rc<SSAOHalfresTarget>,
        target: Rc<LightingTarget>,
        config: RenderingConfig,
        light_info: Rc<RefCell<LightInfo>>,
    ) -> Self {
        LightingPass {
            gl: gl.clone(),
            id,
            config,
            light_info,
            shader: None,
            skybox: None,
            quad: Quad2D::new(gl.clone()),
            view: glam::Mat4::IDENTITY,
            halfres_ssao: ssao_blurred,
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
                self.skybox = None;
            }
            RenderingEvent::ViewUpdated(view) => {
                self.view = view;
            }
            RenderingEvent::SetSkybox(skybox) => {
                self.skybox = Some(skybox);
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
                program.set_uniform(&shader.halfres_ssao, HALFRES_SSAO_INDEX);
                program.set_uniform(&shader.skybox, SKYBOX_INDEX);
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

        // Update lights
        self.light_info.borrow_mut().feed(&self.view, frame);

        Framebuffer::bind(&self.gl, &self.target.fbo);

        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, program);
        #[cfg(feature = "devtools")]
        {
            program.set_uniform(
                &shader.devtools.ssao_enabled,
                self.config.get_is_ssao_enabled() as i32,
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

        // Upload lights
        program.set_uniform(
            &shader.packed_lights_header,
            self.light_info.borrow().header(),
        );
        Texture2D::bind(
            &self.gl,
            &self.light_info.borrow().texture(),
            PACKED_LIGHTS_INDEX as u32,
        );

        self.gbuffer.depth.bind2d(DEPTH_INDEX);
        self.gbuffer.albedo.bind2d(ALBEDO_INDEX);
        self.gbuffer.orm.bind2d(ORM_INDEX);
        self.gbuffer.normal.bind2d(NORMAL_INDEX);
        self.halfres_ssao.texture.bind2d(HALFRES_SSAO_INDEX);

        if let Some(skybox) = &self.skybox {
            let skybox = skybox.cast();
            TextureCube::bind(&self.gl, skybox, SKYBOX_INDEX as u32);
        }

        self.quad.draw(&self.gl)
    }

    #[inline(always)]
    fn end(&mut self, _: &Window, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Framebuffer::unbind(&self.gl);
        Program::unbind(&self.gl);
        Texture2D::unbind(&self.gl, DEPTH_INDEX as u32);
        Texture2D::unbind(&self.gl, ALBEDO_INDEX as u32);
        Texture2D::unbind(&self.gl, ORM_INDEX as u32);
        Texture2D::unbind(&self.gl, NORMAL_INDEX as u32);
        Texture2D::unbind(&self.gl, PACKED_LIGHTS_INDEX as u32);
        Texture2D::unbind(&self.gl, HALFRES_SSAO_INDEX as u32);
        TextureCube::unbind(&self.gl, SKYBOX_INDEX as u32);
        RenderResult::default()
    }
}
