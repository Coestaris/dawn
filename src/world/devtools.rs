use crate::devtools::{DevtoolsToRendererMessage, DevtoolsToWorldMessage, DevtoolsWorldConnection};
use crate::rendering::dispatcher::RenderDispatcher;
use crate::rendering::event::RenderingEvent;
use crate::world::asset::LIGHT_TEXTURE;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_ecs::events::TickEvent;
use dawn_ecs::world::WorldLoopMonitorEvent;
use dawn_graphics::ecs::{
    ObjectAreaLight, ObjectColor, ObjectIntensity, ObjectMesh, ObjectPointLight, ObjectSpotLight,
    ObjectSunLight,
};
use dawn_graphics::gl::raii::texture::Texture;
use dawn_graphics::passes::events::RenderPassEvent;
use dawn_graphics::renderer::RendererMonitorEvent;
use evenio::entity::EntityId;
use evenio::event::Sender;
use evenio::fetch::{Fetcher, Single};
use evenio::prelude::{Query, Receiver};
use evenio::world::World;
use log::info;

#[derive(Query)]
struct SunLightQuery<'a> {
    entity_id: EntityId,
    light: &'a mut ObjectSunLight,
    intensity: &'a mut ObjectIntensity,
    color: &'a mut ObjectColor,
}

pub struct WorldStatistics {
    pub entities: usize,
    pub drawables: usize,
    pub point_lights: usize,
    pub spot_lights: usize,
    pub sun_lights: usize,
    pub area_lights: usize,
}

fn world_monitoring_handler(
    r: Receiver<WorldLoopMonitorEvent>,
    meshes: Fetcher<&ObjectMesh>,
    entities: Fetcher<EntityId>,
    point_lights: Fetcher<&ObjectPointLight>,
    spot_lights: Fetcher<&ObjectSpotLight>,
    sun_lights: Fetcher<&ObjectSunLight>,
    area_lights: Fetcher<&ObjectAreaLight>,
    connection: Single<&mut DevtoolsWorldConnection>,
) {
    let stats = WorldStatistics {
        entities: entities.iter().count(),
        drawables: meshes.iter().count(),
        point_lights: point_lights.iter().count(),
        spot_lights: spot_lights.iter().count(),
        sun_lights: sun_lights.iter().count(),
        area_lights: area_lights.iter().count(),
    };
    let _ = connection
        .sender
        .send(DevtoolsToRendererMessage::WorldMonitor(
            r.event.clone(),
            stats,
        ));
}

fn renderer_monitoring_handler(
    r: Receiver<RendererMonitorEvent>,
    connection: Single<&mut DevtoolsWorldConnection>,
) {
    let _ = connection
        .sender
        .send(DevtoolsToRendererMessage::RendererMonitor(r.event.clone()));
}

fn recv_messages_from_renderer_handler(
    _: Receiver<TickEvent>,
    hub: Single<&mut AssetHub>,
    connection: Single<&mut DevtoolsWorldConnection>,
    mut sun_light_query: Fetcher<SunLightQuery>,
) {
    while let Ok(msg) = connection.receiver.try_recv() {
        match msg {
            DevtoolsToWorldMessage::EnumerateAssets => {
                let mut infos = hub.asset_infos();
                // Sort by type and then by name
                infos.sort_by(|a, b| a.header.id.cmp(&b.header.id));
                let _ = connection
                    .sender
                    .send(DevtoolsToRendererMessage::AssetsEnumerated(infos));
            }

            DevtoolsToWorldMessage::ControlSunlight(control) => {
                for sunlight in sun_light_query.iter_mut() {
                    info!(
                        "Setting sunlight {:?} intensity to {}, color to {:?}, direction to {:?}",
                        sunlight.entity_id, control.intensity, control.color, control.direction
                    );
                    sunlight.intensity.intensity = control.intensity;
                    sunlight.color.color = control.color;
                    sunlight.light.direction = control.direction;
                    sunlight.light.ambient = control.ambient;
                }
            }
        }
    }
}

fn gizmos_assets_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    dispatcher: Single<&mut RenderDispatcher>,
    mut sender: Sender<RenderPassEvent<RenderingEvent>>,
) {
    match r.event {
        AssetHubEvent::AssetLoaded(id) if id.as_str() == LIGHT_TEXTURE => {
            info!("Light texture loaded");
            let texture = hub.get_typed::<Texture>(LIGHT_TEXTURE.into()).unwrap();
            dispatcher.dispatch(RenderingEvent::SetLightTexture(texture), &mut sender);
        }

        _ => {}
    }
}

pub fn setup_devtools_system(world: &mut World, connection: DevtoolsWorldConnection) {
    let id = world.spawn();
    world.insert(id, connection);

    world.add_handler(world_monitoring_handler);
    world.add_handler(renderer_monitoring_handler);
    world.add_handler(recv_messages_from_renderer_handler);

    world.add_handler(gizmos_assets_handler);
}
