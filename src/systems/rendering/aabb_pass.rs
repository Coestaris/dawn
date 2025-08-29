use crate::systems::rendering::CustomPassEvent;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::RendererBackend;

pub(crate) struct AABBPass {
    id: RenderPassTargetId,
}

impl AABBPass {
    pub fn new(id: RenderPassTargetId) -> Self {
        AABBPass { id }
    }
}

impl RenderPass<CustomPassEvent> for AABBPass {
    fn get_target(&self) -> Vec<PassEventTarget<CustomPassEvent>> {
        fn dispatch_aabb_pass(ptr: *mut u8, event: CustomPassEvent) {
            let pass = unsafe { &mut *(ptr as *mut AABBPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_aabb_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: CustomPassEvent) {
        match event {
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "AABBPass"
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<CustomPassEvent>,
        renderable: &Renderable,
    ) -> RenderResult {
        RenderResult::ok(0, 0)
    }
}
