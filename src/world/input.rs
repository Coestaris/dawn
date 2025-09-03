use dawn_graphics::renderer::InputEvent;
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::world::World;
use glam::Vec2;
use std::collections::HashSet;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{Key, PhysicalKey};

#[derive(Component)]
pub struct InputHolder {
    // TODO: Store keys in more efficient way
    key_pressed: HashSet<PhysicalKey>,
    button_pressed: HashSet<MouseButton>,
    mouse_pos: Vec2,
}

impl InputHolder {
    pub fn attach_to_ecs(self, world: &mut World) {
        fn events_handler(r: Receiver<InputEvent>, mut holder: Single<&mut InputHolder>) {
            match &r.event.0 {
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: key,
                            state: ElementState::Released,
                            ..
                        },
                    ..
                } => {
                    holder.key_pressed.remove(key);
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: key,
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    holder.key_pressed.insert(key.clone());
                }
                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button,
                    ..
                } => {
                    holder.button_pressed.remove(button);
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button,
                    ..
                } => {
                    holder.button_pressed.insert(button.clone());
                }
                WindowEvent::CursorMoved { position, .. } => {
                    holder.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
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

    pub fn key_pressed(&self, key: PhysicalKey) -> bool {
        self.key_pressed.contains(&key)
    }

    pub fn button_pressed(&self, button: MouseButton) -> bool {
        self.button_pressed.contains(&button)
    }
}
