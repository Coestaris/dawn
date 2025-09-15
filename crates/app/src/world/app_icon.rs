use crate::world::asset::APPLICATION_ICON_BLOB_ID;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_graphics::renderer::OutputEvent;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use log::info;
use winit::window::Icon;
use crate::assets::blob::Blob;

pub fn map_app_icon_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    mut sender: Sender<OutputEvent>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) if id.as_str() == APPLICATION_ICON_BLOB_ID => {
            info!("Application icon blob loaded");
            let blob = hub
                .get_typed::<Blob>(APPLICATION_ICON_BLOB_ID.into())
                .unwrap();
            let reader = std::io::Cursor::new(&blob.cast().data);
            let icon_dir = ico::IconDir::read(reader).unwrap();
            let icon = icon_dir.entries()[0].decode().unwrap();
            let _ = sender.send(OutputEvent::ChangeIcon(Some(
                Icon::from_rgba(icon.rgba_data().to_vec(), icon.width(), icon.height()).unwrap(),
            )));
        }
        _ => {}
    }
}
