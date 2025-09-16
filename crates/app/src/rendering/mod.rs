use crate::assets::reader::ReaderBackend;
#[cfg(feature = "devtools")]
use crate::devtools::DevtoolsRendererConnection;
use crate::rendering::config::RenderingConfig;
#[cfg(feature = "devtools")]
use crate::rendering::devtools::DevToolsGUI;
use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::{RenderingEvent, RenderingEventMask};
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::fbo::obuffer::OBuffer;
#[cfg(feature = "devtools")]
use crate::rendering::passes::bounding_pass::BoundingPass;
use crate::rendering::passes::geometry_pass::GeometryPass;
#[cfg(feature = "devtools")]
use crate::rendering::passes::gizmos_pass::GizmosPass;
use crate::rendering::passes::lighting_pass::LightingPass;
use crate::rendering::passes::postprocess_pass::PostProcessPass;
use crate::rendering::ubo::camera::CameraUBO;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use crate::world::asset::{
    BILLBOARD_SHADER, GEOMETRY_SHADER, LIGHTING_SHADER, LINE_SHADER, POSTPROCESS_SHADER,
};
use crate::WINDOW_SIZE;
use build_info::BuildInfo;
use dawn_graphics::gl::probe::OpenGLInfo;
use dawn_graphics::passes::events::RenderPassTargetId;
use dawn_graphics::renderer::{CustomRenderer, RendererBackend};
use dawn_graphics::{construct_chain, construct_chain_type};
use glow::HasContext;
use log::{debug, info, warn};
use std::rc::Rc;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::window::Window;

mod config;
#[cfg(feature = "devtools")]
pub mod devtools;
pub mod dispatcher;
pub mod event;
pub mod fallback_tex;
pub mod fbo;
pub mod frustum;
pub mod passes;
pub mod primitive;
mod ubo;

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
        gl.enable(glow::MULTISAMPLE);
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
    devtools_gui: DevToolsGUI,
}

#[cfg(feature = "devtools")]
type ChainType = construct_chain_type!(RenderingEvent; GeometryPass, LightingPass, PostProcessPass, BoundingPass, GizmosPass);
#[cfg(not(feature = "devtools"))]
type ChainType = construct_chain_type!(RenderingEvent; GeometryPass, LightingPass, PostProcessPass);

impl CustomRenderer<ChainType, RenderingEvent> for Renderer {
    fn spawn_chain(
        &mut self,
        w: &Window,
        r: &'static mut RendererBackend<RenderingEvent>,
    ) -> anyhow::Result<ChainType> {
        #[cfg(feature = "devtools")]
        self.devtools_gui.attach_to_window(w, r);

        log_info(&r.info);
        pre_pipeline_construct(&r.gl);

        let gbuffer = Rc::new(GBuffer::new(r.gl.clone(), WINDOW_SIZE));
        let obuffer = Rc::new(OBuffer::new(r.gl.clone(), WINDOW_SIZE));
        let camera_ubo = CameraUBO::new(r.gl.clone(), CAMERA_UBO_BINDING);
        let geometry_pass = GeometryPass::new(
            r.gl.clone(),
            self.ids.geometry_id,
            gbuffer.clone(),
            camera_ubo,
            self.config.clone(),
        );
        let lighting_pass = LightingPass::new(
            r.gl.clone(),
            self.ids.lighting_id,
            gbuffer.clone(),
            obuffer.clone(),
            self.config.clone(),
        );
        let postprocess_pass = PostProcessPass::new(
            r.gl.clone(),
            self.ids.postprocess_id,
            obuffer.clone(),
            self.config.clone(),
        );

        #[cfg(feature = "devtools")]
        {
            let bounding_pass = BoundingPass::new(
                r.gl.clone(),
                self.ids.bounding_id,
                gbuffer.clone(),
                self.config.clone(),
            );
            let gizmo_pass = GizmosPass::new(
                r.gl.clone(),
                self.ids.gizmos_id,
                gbuffer.clone(),
                self.config.clone(),
            );

            Ok(construct_chain!(
                geometry_pass,
                lighting_pass,
                postprocess_pass,
                bounding_pass,
                gizmo_pass
            ))
        }

        #[cfg(not(feature = "devtools"))]
        {
            Ok(construct_chain!(
                geometry_pass,
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
        self.devtools_gui.on_window_event(_window, _event);
    }

    fn before_frame(&mut self, _window: &Window, _backend: &RendererBackend<RenderingEvent>) {
        #[cfg(feature = "devtools")]
        self.devtools_gui.before_frame(_window, _backend);
    }

    fn after_render(&mut self, _window: &Window, _backend: &RendererBackend<RenderingEvent>) {
        #[cfg(feature = "devtools")]
        self.devtools_gui.after_render(_window, _backend);
    }
}

pub struct SetupRenderingParameters {
    #[cfg(feature = "devtools")]
    pub connection: DevtoolsRendererConnection,
    pub reader_backend: Arc<dyn ReaderBackend>,
    pub bi: BuildInfo,
}

pub struct PassIDs {
    pub geometry_id: RenderPassTargetId,
    pub lighting_id: RenderPassTargetId,
    pub postprocess_id: RenderPassTargetId,
    #[cfg(feature = "devtools")]
    pub bounding_id: RenderPassTargetId,
    #[cfg(feature = "devtools")]
    pub gizmos_id: RenderPassTargetId,
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
        let geometry_id = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEW_UPDATED
                | RenderingEventMask::VIEWPORT_RESIZED
                | RenderingEventMask::PERSP_PROJECTION_UPDATED,
            GEOMETRY_SHADER,
        );
        let lighting_id = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEWPORT_RESIZED
                | RenderingEventMask::VIEW_UPDATED,
            LIGHTING_SHADER,
        );
        let postprocess_id = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS | RenderingEventMask::UPDATE_SHADER,
            POSTPROCESS_SHADER,
        );

        #[cfg(feature = "devtools")]
        let bounding_id = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEWPORT_RESIZED,
            LINE_SHADER,
        );
        #[cfg(feature = "devtools")]
        let gizmos_id = dispatcher.pass(
            RenderingEventMask::DROP_ALL_ASSETS
                | RenderingEventMask::UPDATE_SHADER
                | RenderingEventMask::VIEWPORT_RESIZED
                | RenderingEventMask::SET_LIGHT_TEXTURE,
            BILLBOARD_SHADER,
        );

        let config = RenderingConfig::new();
        Self {
            ids: PassIDs {
                geometry_id,
                lighting_id,
                postprocess_id,
                #[cfg(feature = "devtools")]
                bounding_id,
                #[cfg(feature = "devtools")]
                gizmos_id,
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
            devtools_gui: DevToolsGUI::new(
                self.config,
                param.connection,
                param.bi,
                param.reader_backend,
            ),
        }
    }
}
