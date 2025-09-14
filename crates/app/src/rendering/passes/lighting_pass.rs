use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::fbo::obuffer::OBuffer;
use crate::rendering::primitive::quad::Quad;
use crate::rendering::ubo::packed_light::{LightsHeaderCPU, PackedLights};
use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glow::HasContext;
use std::rc::Rc;

const ALBEDO_METALLIC_INDEX: i32 = 0;
const NORMAL_INDEX: i32 = 1;
const PBR_INDEX: i32 = 2;
const DEPTH_INDEX: i32 = 3;
const PACKED_LIGHTS_INDEX: i32 = 4;

struct ShaderContainer {
    shader: TypedAsset<Program>,

    debug_mode: UniformLocation,

    packed_lights_location: UniformLocation,
    packed_lights_header_location: UniformLocation,

    sky_color_location: UniformLocation,
    ground_color_location: UniformLocation,
    diffuse_scale_location: UniformLocation,
    specular_scale_location: UniformLocation,

    albedo_metallic_texture: UniformLocation,
    normal_texture: UniformLocation,
    pbr_texture: UniformLocation,
    depth_texture: UniformLocation,
}

pub(crate) struct LightingPass {
    gl: &'static glow::Context,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<ShaderContainer>,
    quad: Quad,
    view: glam::Mat4,
    packed_lights: PackedLights,
    gbuffer: Rc<GBuffer>,
    obuffer: Rc<OBuffer>,
}

impl LightingPass {
    pub fn new(
        gl: &'static glow::Context,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        obuffer: Rc<OBuffer>,
        config: RenderingConfig,
    ) -> Self {
        LightingPass {
            gl,
            id,
            config,
            shader: None,
            quad: Quad::new(gl),
            view: glam::Mat4::IDENTITY,
            packed_lights: PackedLights::new(gl).unwrap(),
            gbuffer,
            obuffer,
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
            RenderingEvent::UpdateShader(shader) => {
                let clone = shader.clone();
                self.shader = Some(ShaderContainer {
                    shader: clone,
                    debug_mode: shader.cast().get_uniform_location("in_debug_mode").unwrap(),
                    packed_lights_location: shader
                        .cast()
                        .get_uniform_location("in_packed_lights")
                        .unwrap(),
                    packed_lights_header_location: shader
                        .cast()
                        .get_uniform_location("in_packed_lights_header")
                        .unwrap(),
                    sky_color_location: shader
                        .cast()
                        .get_uniform_location("ENV_SKY_COLOR")
                        .unwrap(),
                    ground_color_location: shader
                        .cast()
                        .get_uniform_location("ENV_GROUND_COLOR")
                        .unwrap(),
                    diffuse_scale_location: shader
                        .cast()
                        .get_uniform_location("ENV_DIFFUSE_SCALE")
                        .unwrap(),
                    specular_scale_location: shader
                        .cast()
                        .get_uniform_location("ENV_SPECULAR_SCALE")
                        .unwrap(),
                    albedo_metallic_texture: shader
                        .cast()
                        .get_uniform_location("in_albedo_metallic_texture")
                        .unwrap(),
                    normal_texture: shader
                        .cast()
                        .get_uniform_location("in_normal_texture")
                        .unwrap(),
                    pbr_texture: shader
                        .cast()
                        .get_uniform_location("in_pbr_texture")
                        .unwrap(),
                    depth_texture: shader
                        .cast()
                        .get_uniform_location("in_depth_texture")
                        .unwrap(),
                });

                if let Some(shader) = self.shader.as_mut() {
                    let program = shader.shader.cast();
                    Program::bind(self.gl, &program);
                    program.set_uniform(&shader.albedo_metallic_texture, ALBEDO_METALLIC_INDEX);
                    program.set_uniform(&shader.normal_texture, NORMAL_INDEX);
                    program.set_uniform(&shader.pbr_texture, PBR_INDEX);
                    program.set_uniform(&shader.depth_texture, DEPTH_INDEX);
                    program.set_uniform(&shader.packed_lights_location, PACKED_LIGHTS_INDEX);
                    Program::unbind(self.gl);
                }
            }
            RenderingEvent::ViewportResized(size) => {
                self.gbuffer.resize(size);
                self.obuffer.resize(size);
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
        let header = LightsHeaderCPU::new(lights_count as u32);

        Framebuffer::bind(self.gl, &self.obuffer.fbo);

        let shader = self.shader.as_ref().unwrap();
        let program = shader.shader.cast();
        Program::bind(self.gl, program);
        program.set_uniform(&shader.sky_color_location, self.config.get_sky_color());
        program.set_uniform(
            &shader.ground_color_location,
            self.config.get_ground_color(),
        );
        program.set_uniform(
            &shader.diffuse_scale_location,
            self.config.get_diffuse_scale(),
        );
        program.set_uniform(
            &shader.specular_scale_location,
            self.config.get_specular_scale(),
        );
        program.set_uniform(&shader.debug_mode, self.config.get_output_mode() as i32);
        program.set_uniform(&shader.packed_lights_header_location, header.as_uvec4());
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.albedo_metalic.texture,
            ALBEDO_METALLIC_INDEX as u32,
        );
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.normal.texture,
            NORMAL_INDEX as u32,
        );
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.pbr.texture,
            PBR_INDEX as u32,
        );
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.depth.texture,
            DEPTH_INDEX as u32,
        );
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.packed_lights.texture,
            PACKED_LIGHTS_INDEX as u32,
        );
        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Framebuffer::unbind(self.gl);
        Program::unbind(self.gl);
        Texture::unbind(self.gl, TextureBind::Texture2D, 0);
        RenderResult::default()
    }
}
