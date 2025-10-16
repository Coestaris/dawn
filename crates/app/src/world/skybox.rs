use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::RenderingEvent;
use crate::world::asset::CURRENT_SKYBOX;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_graphics::gl::raii::texture::TextureCube;
use dawn_graphics::passes::events::RenderPassEvent;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;

pub fn map_skybox(
    r: Receiver<AssetHubEvent>,
    hub: Single<&AssetHub>,
    dispatcher: Single<&RenderDispatcher>,
    mut sender: Sender<RenderPassEvent<RenderingEvent>>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(asset) if asset.as_str() == CURRENT_SKYBOX => {
            let skybox = hub.get_typed::<TextureCube>(asset.clone()).unwrap();
            dispatcher.dispatch(RenderingEvent::SetSkybox(skybox.clone()), &mut sender)
        }
        _ => {}
    }
}
