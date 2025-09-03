use crate::rendering::WINDOW_SIZE;
use dawn_graphics::renderer::{InputEvent, ViewEvent};
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
    // TODO!
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
