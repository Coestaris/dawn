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
use crate::run::WINDOW_SIZE;
use crate::world::asset::{
    BILLBOARD_SHADER, GEOMETRY_SHADER, LIGHTING_SHADER, LINE_SHADER, POSTPROCESS_SHADER,
};
use dawn_graphics::gl::probe::OpenGLInfo;
use dawn_graphics::passes::events::RenderPassTargetId;
use dawn_graphics::renderer::{CustomRenderer, RendererBackend};
use dawn_graphics::{construct_chain, construct_chain_type};
use glow::HasContext;
use log::{info, warn};
use std::ops::DerefMut;
use std::rc::Rc;
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
    info!("  Version: {}", info.version);
    info!("  Vendor: {}", info.vendor);
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
    // Pass IDs
    geometry_id: RenderPassTargetId,

    lighting_id: RenderPassTargetId,
    postprocess_id: RenderPassTargetId,
    config: RenderingConfig,

    #[cfg(feature = "devtools")]
    devtools_gui: DevToolsGUI,
    #[cfg(feature = "devtools")]
    bounding_id: RenderPassTargetId,
    #[cfg(feature = "devtools")]
    gizmos_id: RenderPassTargetId,
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

        let gbuffer = Rc::new(GBuffer::new(&r.gl, WINDOW_SIZE));
        let obuffer = Rc::new(OBuffer::new(&r.gl, WINDOW_SIZE));
        let camera_ubo = CameraUBO::new(&r.gl, CAMERA_UBO_BINDING);
        let geometry_pass = GeometryPass::new(
            &r.gl,
            self.geometry_id,
            gbuffer.clone(),
            camera_ubo,
            self.config.clone(),
        );
        let lighting_pass = LightingPass::new(
            &r.gl,
            self.lighting_id,
            gbuffer.clone(),
            obuffer.clone(),
            self.config.clone(),
        );
        let postprocess_pass = PostProcessPass::new(
            &r.gl,
            self.postprocess_id,
            obuffer.clone(),
            self.config.clone(),
        );

        #[cfg(feature = "devtools")]
        {
            let bounding_pass = BoundingPass::new(
                &r.gl,
                self.bounding_id,
                gbuffer.clone(),
                self.config.clone(),
            );
            let gizmo_pass =
                GizmosPass::new(&r.gl, self.gizmos_id, gbuffer.clone(), self.config.clone());

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
}

pub fn setup_rendering(_param: SetupRenderingParameters) -> (RenderDispatcher, Renderer) {
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
    let renderer = Renderer {
        geometry_id,
        lighting_id,

        postprocess_id,
        config: config.clone(),

        #[cfg(feature = "devtools")]
        devtools_gui: DevToolsGUI::new(config.clone(), _param.connection),
        #[cfg(feature = "devtools")]
        gizmos_id,
        #[cfg(feature = "devtools")]
        bounding_id,
    };

    (dispatcher, renderer)
}
