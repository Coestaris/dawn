use glow::HasContext;

pub mod dispatcher;
pub mod event;
pub mod frustum;
pub mod gbuffer;
pub mod passes;
pub mod primitive;

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
