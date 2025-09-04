use dawn_graphics::renderer::{InputEvent, OutputEvent};
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::world::World;
use log::info;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{Key, NamedKey};

#[derive(Component)]
struct CurrentFullscreenState {
    is_fullscreen: bool,
}

pub fn fullscreen_handler(
    r: Receiver<InputEvent>,
    mut state: Single<&mut CurrentFullscreenState>,
    mut s: Sender<(OutputEvent)>,
) {
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
            Key::Named(NamedKey::F11) => {
                info!("Switching fullscreen mode to {}", !state.is_fullscreen);
                s.send(OutputEvent::ChangeFullscreen(!state.is_fullscreen));
                state.is_fullscreen = !state.is_fullscreen;
            }
            _ => {}
        },
        _ => {}
    }
}

pub fn setup_fullscreen_system(world: &mut World) {
    let id = world.spawn();
    world.insert(
        id,
        CurrentFullscreenState {
            is_fullscreen: false,
        },
    );

    world.add_handler(fullscreen_handler);
}
