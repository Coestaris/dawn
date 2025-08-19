use dawn_graphics::input::{InputEvent, KeyCode, MouseButton};
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::world::World;
use glam::Vec2;
use std::collections::HashSet;

#[derive(Component)]
pub struct InputHolder {
    // TODO: Store keys in more efficient way
    key_pressed: HashSet<KeyCode>,
    button_pressed: HashSet<MouseButton>,
    mouse_pos: Vec2,
}

impl InputHolder {
    pub fn attach_to_ecs(self, world: &mut World) {
        fn events_handler(ie: Receiver<InputEvent>, mut holder: Single<&mut InputHolder>) {
            match ie.event {
                InputEvent::KeyPress(key) => {
                    holder.key_pressed.insert(key.clone());
                }
                InputEvent::KeyRelease(key) => {
                    holder.key_pressed.remove(&key);
                }
                InputEvent::MouseButtonPress(button) => {
                    holder.button_pressed.insert(button.clone());
                }
                InputEvent::MouseButtonRelease(button) => {
                    holder.button_pressed.remove(&button);
                }
                InputEvent::MouseMove { x, y } => {
                    holder.mouse_pos = Vec2::new(*x, *y);
                }
                _ => {}
            }
        }

        let entity = world.spawn();
        world.insert(entity, self);
        world.add_handler(events_handler.low());
    }

    pub fn new() -> Self {
        Self {
            key_pressed: HashSet::new(),
            button_pressed: HashSet::new(),
            mouse_pos: Default::default(),
        }
    }

    pub fn mouse_pos(&self) -> Vec2 {
        self.mouse_pos
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.key_pressed.contains(&key)
    }

    pub fn button_pressed(&self, button: MouseButton) -> bool {
        self.button_pressed.contains(&button)
    }
}
