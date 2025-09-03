use crate::world::asset_swap::{AndThen, DropAllAssetsEvent};
use dawn_graphics::renderer::InputEvent;
use evenio::event::{Receiver, Sender};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{Key, NamedKey};

pub fn escape_handler(r: Receiver<InputEvent>, mut s: Sender<DropAllAssetsEvent>) {
    // info!("Input event: {:?}", r.event);
    match &r.event.0 {
        WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    logical_key: key,
                    state: ElementState::Released,
                    ..
                },
            ..
        } => match key.as_ref() {
            Key::Named(NamedKey::Escape) => {
                s.send(DropAllAssetsEvent(AndThen::StopMainLoop));
            }
            Key::Named(NamedKey::F5) => {
                s.send(DropAllAssetsEvent(AndThen::ReloadAssets));
            }
            _ => {}
        },
        _ => {}
    }
}
