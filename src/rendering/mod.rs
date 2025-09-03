use dawn_graphics::gl::bindings;

pub mod dispatcher;
pub mod event;
pub mod frustum;
pub mod gbuffer;
pub mod passes;
pub mod primitive;

pub fn pre_pipeline_construct() {
    // Setup OpenGL state
    unsafe {
        // Enable wireframe mode
        bindings::Enable(bindings::DEPTH_TEST);
        bindings::DepthFunc(bindings::LEQUAL);
        bindings::Enable(bindings::MULTISAMPLE);
        bindings::Hint(bindings::PERSPECTIVE_CORRECTION_HINT, bindings::NICEST);
        bindings::Enable(bindings::BLEND);
        bindings::BlendFunc(bindings::SRC_ALPHA, bindings::ONE_MINUS_SRC_ALPHA);
    }
}
