use crate::rendering::WINDOW_SIZE;
use dawn_graphics::input::{InputEvent, KeyCode};
use dawn_graphics::renderer::ViewEvent;
use dawn_graphics::view::ViewGeometry;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::prelude::World;

#[derive(Component)]
struct CurrentGeometry {
    is_fullscreen: bool,
}

fn fullscreen_handler(
    ie: Receiver<InputEvent>,
    mut cg: Single<&mut CurrentGeometry>,
    mut sender: Sender<ViewEvent>,
) {
    match ie.event {
        InputEvent::KeyPress(KeyCode::Function(11)) => {
            cg.is_fullscreen = !cg.is_fullscreen;
            if cg.is_fullscreen {
                sender.send(ViewEvent::SetGeometry(ViewGeometry::BorderlessFullscreen));
            } else {
                sender.send(ViewEvent::SetGeometry(ViewGeometry::Normal(WINDOW_SIZE)));
            }
        }
        _ => {}
    }
}

pub fn setup_fullscreen_system(world: &mut World) {
    let e = world.spawn();
    world.insert(
        e,
        CurrentGeometry {
            is_fullscreen: false,
        },
    );

    world.add_handler(fullscreen_handler);
}
