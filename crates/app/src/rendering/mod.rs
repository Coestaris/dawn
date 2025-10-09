use crate::assets::reader::ReaderBackend;
#[cfg(feature = "devtools")]
use crate::devtools::DevtoolsRendererConnection;
use crate::rendering::config::RenderingConfig;
#[cfg(feature = "devtools")]
use crate::rendering::devtools::DevToolsGUI;
use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::{RenderingEvent, RenderingEventMask};
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::fbo::halfres::HalfresBuffer;
use crate::rendering::fbo::obuffer::LightningTarget;
use crate::rendering::fbo::ssao::{SSAOHalfresTarget, SSAOTarget};
#[cfg(feature = "devtools")]
use crate::rendering::passes::devtools_pass::DevtoolsPass;
use crate::rendering::passes::forward_pass::ForwardPass;
use crate::rendering::passes::lighting_pass::LightingPass;
use crate::rendering::passes::postprocess_pass::PostProcessPass;
use crate::rendering::passes::ssao_blur::SSAOBlurPass;
use crate::rendering::passes::ssao_halfres::SSAOHalfresPass;
use crate::rendering::passes::ssao_raw::SSAORawPass;
use crate::rendering::shaders::{
    BILLBOARD_SHADER, FORWARD_SHADER, LIGHTING_SHADER, LINE_SHADER, POSTPROCESS_SHADER,
    SSAO_BLUR_SHADER, SSAO_HALFRES_SHADER, SSAO_RAW_SHADER,
};
use crate::rendering::ubo::camera::CameraUBO;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use crate::WINDOW_SIZE;
use build_info::BuildInfo;
use dawn_graphics::gl::probe::OpenGLInfo;
use dawn_graphics::passes::events::RenderPassTargetId;
use dawn_graphics::renderer::{CustomRenderer, RendererBackend};
use dawn_graphics::{construct_chain, construct_chain_type};
use glam::{Vec3, Vec4};
use glow::HasContext;
use log::{debug, info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::window::Window;

mod config;
#[cfg(feature = "devtools")]
pub mod devtools;
pub mod dispatcher;
pub mod event;
pub mod fbo;
pub mod frustum;
pub mod passes;
pub mod primitive;
pub mod shaders;
pub mod textures;
pub mod ubo;
pub mod preprocessor;

fn log_info(info: &OpenGLInfo) {
    info!("OpenGL information:");
    info!(
        "{}.{}.{} {} {}",
        info.version.major,
        info.version.minor,
        info.version.revision.unwrap_or(0),
        if info.version.is_embedded {
            "ES"
        } else {
            "Core"
        },
        info.version.vendor_info
    );
    info!("  Renderer: {}", info.renderer);
    if let Some(shading_lang_version) = &info.shading_language_version {
        info!("  Shading Language Version: {}", shading_lang_version);
    } else {
        warn!("  Shading Language Version: Unknown");
    }
    info!("  Number of Extensions: {}", info.extensions.len());
    info!("  Number of Binary Formats: {}", info.binary_formats.len());
    if let Some(depth) = info.depth_bits {
        info!("  Depth Bits: {}", depth);
    } else {
        warn!("  Depth Bits: Unknown");
    }
    if let Some(stencil) = info.stencil_bits {
        info!("  Stencil Bits: {}", stencil);
    } else {
        warn!("  Stencil Bits: Unknown");
    }
}

fn pre_pipeline_construct(gl: &glow::Context) {
    // Setup OpenGL state
    unsafe {
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LEQUAL);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        // gl.enable(glow::CULL_FACE);
        // gl.cull_face(glow::BACK);
        // gl.front_face(glow::CCW);
    }
}


pub struct Renderer {
    ids: PassIDs,
    config: RenderingConfig,
    #[cfg(feature = "devtools")]
    devtools_gui: Rc<RefCell<DevToolsGUI>>,
}

#[cfg(feature = "devtools")]
type ChainType = construct_chain_type!(RenderingEvent; ForwardPass, SSAOHalfresPass, SSAORawPass, SSAOBlurPass, LightingPass, PostProcessPass, DevtoolsPass);
#[cfg(not(feature = "devtools"))]
type ChainType = construct_chain_type!(RenderingEvent; ForwardPass, SSAOHalfresPass, SSAORawPass, SSAOBlurPass, LightingPass, PostProcessPass);

impl CustomRenderer<ChainType, RenderingEvent> for Renderer {
    fn spawn_chain(
        &mut self,
        w: &Window,
        r: &'static mut RendererBackend<RenderingEvent>,
    ) -> anyhow::Result<ChainType> {
        #[cfg(feature = "devtools")]
        self.devtools_gui.borrow_mut().attach_to_window(w, r);

        log_info(&r.info);
        pre_pipeline_construct(&r.gl);

        let gbuffer = Rc::new(GBuffer::new(r.gl.clone(), WINDOW_SIZE).unwrap());
        let halfres = Rc::new(HalfresBuffer::new(r.gl.clone(), WINDOW_SIZE).unwrap());
        let lighting_taget = Rc::new(LightningTarget::new(r.gl.clone(), WINDOW_SIZE).unwrap());
        let ssao_raw_target = Rc::new(SSAOHalfresTarget::new(r.gl.clone(), WINDOW_SIZE).unwrap());
        let ssao_blur_target = Rc::new(SSAOTarget::new(r.gl.clone(), WINDOW_SIZE).unwrap());

        let camera_ubo = CameraUBO::new(r.gl.clone(), CAMERA_UBO_BINDING);

        let forward_pass = ForwardPass::new(
            r.gl.clone(),
            self.ids.forward_id,
            gbuffer.clone(),
            camera_ubo,
            self.config.clone(),
        );
        let ssao_halfres = SSAOHalfresPass::new(
            r.gl.clone(),
            self.ids.ssao_halfres,
            gbuffer.clone(),
            halfres.clone(),
        );
        let ssao_raw = SSAORawPass::new(
            r.gl.clone(),
            self.ids.ssao_raw,
            halfres.clone(),
            ssao_raw_target.clone(),
            self.config.clone(),
        );
        let ssao_blur = SSAOBlurPass::new(
            r.gl.clone(),
            self.ids.ssao_blur,
            gbuffer.clone(),
            ssao_raw_target.clone(),
            ssao_blur_target.clone(),
            self.config.clone(),
        );
        let lighting_pass = LightingPass::new(
            r.gl.clone(),
            self.ids.lighting_id,
            gbuffer.clone(),
            ssao_blur_target.clone(),
            lighting_taget.clone(),
            self.config.clone(),
        );
        let postprocess_pass = PostProcessPass::new(
            r.gl.clone(),
            self.ids.postprocess_id,
            lighting_taget.clone(),
            self.config.clone(),
        );

        #[cfg(feature = "devtools")]
        {
            let devtools_pass = DevtoolsPass::new(
                r.gl.clone(),
                self.ids.devtools_id,
                gbuffer.clone(),
                self.config.clone(),
                self.devtools_gui.clone(),
            );

            Ok(construct_chain!(
                forward_pass,
                ssao_halfres,
                ssao_raw,
                ssao_blur,
                lighting_pass,
                postprocess_pass,
                devtools_pass
            ))
        }

        #[cfg(not(feature = "devtools"))]
        {
            Ok(construct_chain!(
                forward_pass,
                ssao_halfres,
                ssao_raw,
                ssao_blur,
                lighting_pass,
                postprocess_pass
            ))
        }
    }

    fn on_window_event(
        &mut self,
        _window: &Window,
        _backend: &RendererBackend<RenderingEvent>,
        _event: &WindowEvent,
    ) {
        debug!("Renderer received window event: {:?}", _event);

        #[cfg(feature = "devtools")]
        self.devtools_gui
            .borrow_mut()
            .on_window_event(_window, _event);
    }

    fn before_frame(&mut self, _window: &Window, _backend: &RendererBackend<RenderingEvent>) {
        #[cfg(feature = "devtools")]
        self.devtools_gui
            .borrow_mut()
            .before_frame(_window, _backend);
    }
}

pub struct SetupRenderingParameters {
    #[cfg(feature = "devtools")]
    pub connection: DevtoolsRendererConnection,
    pub reader_backend: Arc<dyn ReaderBackend>,
    pub bi: BuildInfo,
}

pub struct PassIDs {
    pub forward_id: RenderPassTargetId,
    pub ssao_halfres: RenderPassTargetId,
    pub ssao_raw: RenderPassTargetId,
    pub ssao_blur: RenderPassTargetId,
    pub lighting_id: RenderPassTargetId,
    pub postprocess_id: RenderPassTargetId,
    #[cfg(feature = "devtools")]
    pub devtools_id: RenderPassTargetId,
}

pub struct RendererBuilder {
    ids: PassIDs,
    config: RenderingConfig,
    dispatcher: RenderDispatcher,
}

impl RendererBuilder {
    pub fn new() -> Self {
        // Allocate the render pass IDs and select the events they will respond to.
        // This must be done before creating the renderer, because the passes
        // will need the IDs during their construction.
        let mut dispatcher = RenderDispatcher::new();
        let forward_id = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEW_UPDATED
                | RenderingEventMask::VIEWPORT_RESIZED
                | RenderingEventMask::PERSP_PROJECTION_UPDATED,
            &[FORWARD_SHADER],
        );
        let ssao_halfres = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEWPORT_RESIZED,
            &[SSAO_HALFRES_SHADER],
        );
        let ssao_raw = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEWPORT_RESIZED,
            &[SSAO_RAW_SHADER],
        );
        let ssao_blur = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEWPORT_RESIZED,
            &[SSAO_BLUR_SHADER],
        );
        let lighting_id = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEWPORT_RESIZED
                | RenderingEventMask::VIEW_UPDATED,
            &[LIGHTING_SHADER],
        );
        let postprocess_id = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS | RenderingEventMask::UPDATE_SHADER,
            &[POSTPROCESS_SHADER],
        );

        #[cfg(feature = "devtools")]
        let devtools_id = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEWPORT_RESIZED
                | RenderingEventMask::SET_LIGHT_TEXTURE
                | RenderingEventMask::VIEW_UPDATED
                | RenderingEventMask::PERSP_PROJECTION_UPDATED,
            &[LINE_SHADER, BILLBOARD_SHADER],
        );

        let config = RenderingConfig::new();
        Self {
            ids: PassIDs {
                forward_id,
                ssao_halfres,
                ssao_raw,
                ssao_blur,
                lighting_id,
                postprocess_id,
                #[cfg(feature = "devtools")]
                devtools_id,
            },

            config,
            dispatcher,
        }
    }

    pub fn build_dispatcher(&self) -> RenderDispatcher {
        self.dispatcher.clone()
    }

    pub fn build_renderer(self, param: SetupRenderingParameters) -> Renderer {
        Renderer {
            ids: self.ids,
            config: self.config.clone(),
            #[cfg(feature = "devtools")]
            devtools_gui: Rc::new(RefCell::new(DevToolsGUI::new(
                self.config,
                param.connection,
                param.bi,
                param.reader_backend,
            ))),
        }
    }
}
