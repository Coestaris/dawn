use crate::rendering::event::{RenderingEvent, RenderingEventMask};
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::AssetID;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::passes::events::{RenderPassEvent, RenderPassTargetId};
use dawn_graphics::renderer::InputEvent;
use egui::ahash::HashSet;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::prelude::World;
use glam::{Mat4, UVec2};
use log::info;
use winit::event::WindowEvent;

#[derive(Clone)]
struct PassDescriptor {
    id: RenderPassTargetId,
    events: RenderingEventMask,
    shaders: HashSet<AssetID>,
}

#[derive(Component, Clone)]
pub struct RenderDispatcher {
    pub descriptors: Vec<PassDescriptor>,
    pub perspective_projection: Mat4,
    pub ortho_projection: Mat4,
}

impl RenderDispatcher {
    pub fn new() -> Self {
        Self {
            descriptors: Vec::new(),
            perspective_projection: Mat4::IDENTITY,
            ortho_projection: Mat4::IDENTITY,
        }
    }

    fn recalculate_projection(&mut self, screen: UVec2) {
        let aspect = screen.x as f32 / screen.y as f32;
        self.perspective_projection =
            Mat4::perspective_rh_gl(std::f32::consts::FRAC_PI_3, aspect, 0.1, 100.0);
        self.ortho_projection =
            Mat4::orthographic_rh_gl(0.0, screen.x as f32, screen.y as f32, 0.0, -1.0, 1.0);
    }

    pub fn pass(&mut self, events: RenderingEventMask, shaders: &[&str]) -> RenderPassTargetId {
        let id = RenderPassTargetId::new();
        self.descriptors.push(PassDescriptor {
            id,
            events,
            shaders: shaders.iter().map(|s| s.to_string().into()).collect(),
        });
        id
    }

    pub fn dispatch(
        &self,
        event: RenderingEvent,
        sender: &mut Sender<RenderPassEvent<RenderingEvent>>,
    ) {
        let bit = match event {
            RenderingEvent::DropAllAssets => RenderingEventMask::DROP_ALL_ASSETS,
            RenderingEvent::UpdateShader(_, _) => RenderingEventMask::UPDATE_SHADER,
            RenderingEvent::ViewUpdated(_) => RenderingEventMask::UPDATE_SHADER,
            RenderingEvent::PerspectiveProjectionUpdated(_) => {
                RenderingEventMask::PERSP_PROJECTION_UPDATED
            }
            RenderingEvent::OrthographicProjectionUpdated(_) => {
                RenderingEventMask::ORTHO_PROJECTION_UPDATED
            }
            RenderingEvent::ViewportResized(_) => RenderingEventMask::VIEWPORT_RESIZED,

            RenderingEvent::SetLightTexture(_) => RenderingEventMask::SET_LIGHT_TEXTURE,
        };

        for descriptor in self.descriptors.iter() {
            if descriptor.events.contains(bit) {
                sender.send(RenderPassEvent::new(descriptor.id, event.clone()));
            }
        }
    }

    /// Translates the AssetHub events to the passes
    pub fn dispatch_assets(
        &self,
        event: &AssetHubEvent,
        hub: &mut AssetHub,
        mut sender: Sender<RenderPassEvent<RenderingEvent>>,
    ) {
        if let AssetHubEvent::AssetLoaded(aid) = event {
            for descriptor in self.descriptors.iter() {
                if descriptor.shaders.contains(&aid)
                    && descriptor
                        .events
                        .contains(RenderingEventMask::UPDATE_SHADER)
                {
                    let shader = hub.get_typed::<Program>(aid.clone()).unwrap();
                    sender.send(RenderPassEvent::new(
                        descriptor.id,
                        RenderingEvent::UpdateShader(aid.clone(), shader),
                    ));
                }
            }
        }
    }

    pub fn dispatch_update_view(
        &self,
        view: Mat4,
        mut sender: Sender<RenderPassEvent<RenderingEvent>>,
    ) {
        self.dispatch(RenderingEvent::ViewUpdated(view), &mut sender);
    }

    pub fn dispatch_drop_assets(&self, mut sender: Sender<RenderPassEvent<RenderingEvent>>) {
        self.dispatch(RenderingEvent::DropAllAssets, &mut sender);
    }

    /// Translates the Input events to the passes
    pub fn dispatch_input(
        &mut self,
        event: &InputEvent,
        mut sender: Sender<RenderPassEvent<RenderingEvent>>,
    ) {
        match &event.0 {
            WindowEvent::Resized(size) => {
                info!("Viewport resized to {:?}", size);

                let size = UVec2::new(size.width as u32, size.height as u32);
                self.recalculate_projection(size);
                self.dispatch(
                    RenderingEvent::PerspectiveProjectionUpdated(self.perspective_projection),
                    &mut sender,
                );
                self.dispatch(
                    RenderingEvent::OrthographicProjectionUpdated(self.ortho_projection),
                    &mut sender,
                );
                self.dispatch(RenderingEvent::ViewportResized(size), &mut sender);
            }
            _ => {}
        }
    }

    pub(crate) fn attach_to_ecs(self, world: &mut World, win_size: UVec2) {
        fn asset_events_handler(
            r: Receiver<AssetHubEvent>,
            hub: Single<&mut AssetHub>,
            dispatcher: Single<&RenderDispatcher>,
            sender: Sender<RenderPassEvent<RenderingEvent>>,
        ) {
            dispatcher.dispatch_assets(r.event, hub.0, sender);
        }

        fn input_events_handler(
            r: Receiver<InputEvent>,
            mut dispatcher: Single<&mut RenderDispatcher>,
            sender: Sender<RenderPassEvent<RenderingEvent>>,
        ) {
            dispatcher.dispatch_input(r.event, sender);
        }

        let entity = world.spawn();
        world.insert(entity, self);

        world.add_handler(asset_events_handler);
        world.add_handler(input_events_handler);
    }
}
