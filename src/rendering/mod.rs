use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::{RenderingEvent, RenderingEventMask};
use crate::rendering::gbuffer::GBuffer;
use crate::rendering::passes::bounding_pass::BoundingPass;
use crate::rendering::passes::geometry_pass::GeometryPass;
use crate::rendering::passes::screen_pass::ScreenPass;
use crate::run::WINDOW_SIZE;
use crate::ui::UIRendererConnection;
use dawn_graphics::passes::chain::RenderChain;
use dawn_graphics::passes::events::RenderPassTargetId;
use dawn_graphics::renderer::{CustomRenderer, RendererBackend};
use dawn_graphics::{construct_chain, construct_chain_type};
use glow::HasContext;
use imgui_glow_renderer::AutoRenderer;
use imgui_winit_support::WinitPlatform;
use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::Rc;
use std::time::Instant;
use winit::event::{Event, WindowEvent};
use winit::window::Window;
use crate::rendering::ui::RenderingConfig;

pub mod dispatcher;
pub mod event;
pub mod fallback_tex;
pub mod frustum;
pub mod gbuffer;
pub mod passes;
pub mod primitive;
mod ui;

pub fn pre_pipeline_construct(gl: &glow::Context) {
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
    bounding_id: RenderPassTargetId,
    screen_id: RenderPassTargetId,

    last_frame: Instant,
    imgui: Rc<RefCell<imgui::Context>>,
    imgui_winit: WinitPlatform,
    ig_render: Option<AutoRenderer>,
    ui: ui::UI,
    config: RenderingConfig,
}

type ChainType = construct_chain_type!(RenderingEvent; GeometryPass, ScreenPass, BoundingPass);

impl CustomRenderer<ChainType, RenderingEvent> for Renderer {
    fn spawn_chain(
        &mut self,
        w: &Window,
        r: &'static mut RendererBackend<RenderingEvent>,
    ) -> anyhow::Result<ChainType> {
        let imgui_context = r.new_context()?;
        let mut imgui = self.imgui.borrow_mut();
        self.ig_render = Some(AutoRenderer::new(imgui_context, imgui.deref_mut())?);
        drop(imgui);
        self.imgui_winit.attach_window(
            self.imgui.borrow_mut().io_mut(),
            w,
            imgui_winit_support::HiDpiMode::Default,
        );

        pre_pipeline_construct(&r.gl);

        let gbuffer = Rc::new(GBuffer::new(&r.gl, WINDOW_SIZE));
        let geometry_pass = GeometryPass::new(&r.gl, self.geometry_id, gbuffer.clone(), self.config.clone());
        let bounding_pass = BoundingPass::new(&r.gl, self.bounding_id, gbuffer.clone());
        let screen_pass = ScreenPass::new(&r.gl, self.screen_id, gbuffer.clone(), self.config.clone());

        Ok(construct_chain!(geometry_pass, screen_pass, bounding_pass,))
    }

    fn on_window_event(
        &mut self,
        window: &Window,
        _backend: &RendererBackend<RenderingEvent>,
        event: &WindowEvent,
    ) {
        self.imgui_winit.handle_event::<()>(
            self.imgui.borrow_mut().io_mut(),
            window,
            // Fake the event to be a winit::event::Event for imgui_winit_support
            &Event::<()>::WindowEvent {
                window_id: window.id(),
                event: event.clone(),
            },
        );
    }

    fn before_frame(&mut self, window: &Window, _backend: &RendererBackend<RenderingEvent>) {
        let now = Instant::now();
        self.imgui
            .borrow_mut()
            .io_mut()
            .update_delta_time(now - self.last_frame);
        self.last_frame = now;

        self.imgui_winit
            .prepare_frame(self.imgui.borrow_mut().io_mut(), &window)
            .unwrap();
    }

    fn after_render(&mut self, window: &Window, _backend: &RendererBackend<RenderingEvent>) {
        if let Some(renderer) = &mut self.ig_render {
            // Render UI here if needed
            let mut imgui = self.imgui.borrow_mut();
            let ui = imgui.frame();
            self.imgui_winit.prepare_render(ui, &window);

            self.ui.render(ui);

            let draw_data = imgui.render();
            unsafe {
                renderer.render(draw_data).unwrap();
            }
        }
    }
}

pub fn setup_rendering(ui: UIRendererConnection) -> (RenderDispatcher, Renderer) {
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
        "geometry_shader",
    );
    let bounding_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::VIEW_UPDATED
            | RenderingEventMask::VIEWPORT_RESIZED
            | RenderingEventMask::PERSP_PROJECTION_UPDATED,
        "bounding_shader",
    );
    let screen_id = dispatcher.pass(
        RenderingEventMask::DROP_ALL_ASSETS
            | RenderingEventMask::UPDATE_SHADER
            | RenderingEventMask::VIEWPORT_RESIZED,
        "screen_shader",
    );

    let config = RenderingConfig::new();
    let mut imgui = imgui::Context::create();
    let renderer = Renderer {
        geometry_id,
        bounding_id,
        screen_id,

        imgui_winit: WinitPlatform::new(&mut imgui),
        imgui: Rc::new(RefCell::new(imgui)),
        last_frame: Instant::now(),
        ig_render: None,
        config: config.clone(),
        ui: ui::UI::new(config.clone(), ui),
    };

    (dispatcher, renderer)
}
