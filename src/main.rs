mod chain;
mod logging;

use crate::chain::{AABBPass, CustomPassEvent, GeometryPass};
use crate::logging::{format_system_time, CommonLogger};
use dawn_assets::factory::FactoryBinding;
use dawn_assets::hub::{AssetHub, AssetHubEvent};
use dawn_assets::ir::IRAsset;
use dawn_assets::reader::AssetReader;
use dawn_assets::{AssetHeader, AssetID, AssetType};
use dawn_ecs::{run_loop_with_monitoring, MainLoopMonitoring, StopEventLoop, Tick};
use dawn_graphics::construct_chain;
use dawn_graphics::gl::entities::mesh::Mesh;
use dawn_graphics::gl::entities::shader_program::ShaderProgram;
use dawn_graphics::gl::entities::texture::Texture;
use dawn_graphics::input::{InputEvent, KeyCode};
use dawn_graphics::passes::chain::ChainCons;
use dawn_graphics::passes::chain::ChainNil;
use dawn_graphics::passes::events::{RenderPassEvent, RenderPassTargetId};
use dawn_graphics::passes::pipeline::RenderPipeline;
use dawn_graphics::renderable::{Position, RenderableMesh, Rotation, Scale};
use dawn_graphics::renderer::{Renderer, RendererBackendConfig, RendererMonitoring};
use dawn_graphics::view::{PlatformSpecificViewConfig, ViewConfig};
use dawn_yarc::Manifest;
use evenio::component::Component;
use evenio::event::{Insert, Receiver, Sender, Spawn};
use evenio::fetch::{Fetcher, Single};
use evenio::world::World;
use glam::*;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fmt, str::FromStr};

/// Small helper to parse env vars with defaults
fn env_parse<T: FromStr>(key: &str, default: T) -> T
where
    <T as FromStr>::Err: fmt::Display,
{
    match env::var(key) {
        Ok(s) => match s.parse::<T>() {
            Ok(v) => v,
            Err(e) => {
                warn!("ENV {} parse error ({}). Using default: {:?}", key, e, default);
                default
            }
        },
        Err(_) => default,
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(s) => matches!(s.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default,
    }
}

/// Detect target refresh rate with OS defaults and optional override
fn detect_refresh_rate() -> f32 {
    // Manual override takes precedence (e.g. DAWN_REFRESH=120)
    if let Ok(val) = env::var("DAWN_REFRESH") {
        if let Ok(f) = val.trim().parse::<f32>() {
            if f > 0.0 {
                return f;
            }
        }
        warn!("DAWN_REFRESH invalid ('{}'), falling back to OS defaults", val);
    }
    #[cfg(target_os = "linux")]
    {
        60.0
    }
    #[cfg(not(target_os = "linux"))]
    {
        144.0
    }
}

/// Parse DAWN_WINDOW="WIDTHxHEIGHT"
fn parse_window_size(default_w: u32, default_h: u32) -> (u32, u32) {
    if let Ok(s) = env::var("DAWN_WINDOW") {
        let parts: Vec<_> = s.to_lowercase().split('x').collect();
        if parts.len() == 2 {
            if let (Ok(w), Ok(h)) = (parts[0].trim().parse::<u32>(), parts[1].trim().parse::<u32>()) {
                if w > 0 && h > 0 {
                    return (w, h);
                }
            }
        }
        warn!("DAWN_WINDOW invalid ('{}'), expected e.g. 1280x720. Using default.", s);
    }
    (default_w, default_h)
}

#[derive(Component)]
struct GameController {
    geometry_pass_id: RenderPassTargetId,
    aabb_pass_id: RenderPassTargetId,
}

/// Optional spin controller for entities (user‑adjustable via key input).
#[derive(Component, Clone, Copy)]
struct Spin {
    speed_x: f32,
    speed_y: f32,
    paused: bool,
}
impl Default for Spin {
    fn default() -> Self {
        Self {
            speed_x: 0.5,
            speed_y: 1.0,
            paused: false,
        }
    }
}

impl GameController {
    fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
    }

    /// Resolve the YARC asset path with a flexible strategy:
    /// 1) DAWN_YARC env var
    /// 2) compile‑time option_env!("YARC_FILE")
    /// 3) common defaults: ./assets/main.yarc or ./assets.yarc
    fn resolve_yarc_path() -> Result<PathBuf, String> {
        if let Ok(from_env) = env::var("DAWN_YARC") {
            let p = PathBuf::from(from_env);
            if p.exists() {
                return Ok(p);
            } else {
                return Err(format!("DAWN_YARC points to missing file: {:?}", p));
            }
        }
        if let Some(ct) = option_env!("YARC_FILE") {
            let p = PathBuf::from(ct);
            if p.exists() {
                return Ok(p);
            }
        }
        for candidate in [
            PathBuf::from("assets/main.yarc"),
            PathBuf::from("assets.yarc"),
            PathBuf::from("./main.yarc"),
        ] {
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        Err("No YARC file found. Set DAWN_YARC, or ensure assets/main.yarc exists.".into())
    }

    pub fn setup_asset_hub(world: &mut World) -> (FactoryBinding, FactoryBinding, FactoryBinding) {
        struct Reader;
        impl AssetReader for Reader {
            fn read(&mut self) -> Result<HashMap<AssetID, (AssetHeader, IRAsset)>, String> {
                let yarc_path = GameController::resolve_yarc_path()?;
                info!("Reading assets from: {}", yarc_path.display());

                let (manifest, assets) = dawn_yarc::read(yarc_path.clone())
                    .map_err(|e| format!("Failed to read YARC '{}': {}", yarc_path.display(), e))?;

                #[rustfmt::skip]
                fn log(manifest: Manifest) {
                    debug!("> Version     : {}", manifest.version.unwrap_or("unknown".to_string()));
                    debug!("> Author      : {}", manifest.author.unwrap_or("unknown".to_string()));
                    debug!("> Description : {}", manifest.description.unwrap_or("No description".to_string()));
                    debug!("> License     : {}", manifest.license.unwrap_or("No license".to_string()));
                    if let Ok(ts) = format_system_time(manifest.created) {
                        debug!("> Created     : {}", ts);
                    }
                    debug!("> Tool        : {} (version {})", manifest.tool, manifest.tool_version);
                    debug!("> Serializer  : {} (version {})", manifest.serializer, manifest.serializer_version);
                    debug!("> Assets      : {}", manifest.headers.len());
                }
                log(manifest);

                let mut result = HashMap::new();
                for (header, ir) in assets {
                    result.insert(header.id.clone(), (header, ir));
                }
                Ok(result)
            }
        }

        let mut hub =
            AssetHub::new(Reader).map_err(|e| error!("AssetHub init failed: {}", e)).unwrap();

        // Factories bound directly to the renderer
        let shader_binding = hub.create_factory_biding(AssetType::Shader);
        let texture_binding = hub.create_factory_biding(AssetType::Texture);
        let mesh_binding = hub.create_factory_biding(AssetType::Mesh);

        // Kick off loading. Errors are handled in the asset event handlers.
        if let Err(e) = hub.query_load_all() {
            error!("query_load_all failed: {}", e);
        }
        hub.attach_to_ecs(world);

        (shader_binding, texture_binding, mesh_binding)
    }

    pub fn setup_graphics(
        world: &mut World,
        shader_binding: FactoryBinding,
        texture_binding: FactoryBinding,
        mesh_binding: FactoryBinding,
    ) -> (RenderPassTargetId, RenderPassTargetId) {
        let (win_w, win_h) = parse_window_size(800, 600);
        let title = env::var("DAWN_TITLE").unwrap_or_else(|_| "Hello world".to_string());
        let vsync = env_bool("DAWN_VSYNC", true);
        let fps = detect_refresh_rate();

        info!(
            "View: '{}' {}x{} | vsync={} | target fps={}",
            title, win_w, win_h, vsync, fps
        );

        let view_config = ViewConfig {
            platform_specific: PlatformSpecificViewConfig {},
            title,
            width: win_w,
            height: win_h,
        };

        let backend_config = RendererBackendConfig {
            fps: fps as usize,
            shader_factory_binding: Some(shader_binding),
            texture_factory_binding: Some(texture_binding),
            mesh_factory_binding: Some(mesh_binding),
            vsync,
        };

        let geometry_pass_id = RenderPassTargetId::new();
        let aabb_pass_id = RenderPassTargetId::new();

        // Construct the pipeline with both passes enabled; simple and robust.
        let renderer = Renderer::new_with_monitoring(view_config, backend_config, move |_| {
            let geometry_pass = GeometryPass::new(geometry_pass_id, (win_w as i32, win_h as i32));
            let aabb_pass = AABBPass::new(aabb_pass_id);
            Ok(RenderPipeline::new(construct_chain!(geometry_pass, aabb_pass)))
        })
        .expect("Renderer init failed");
        renderer.attach_to_ecs(world);

        (geometry_pass_id, aabb_pass_id)
    }

    pub fn setup(world: &mut World) {
        let (shader, texture, mesh) = Self::setup_asset_hub(world);
        let (geometry_pass, aabb_pass) = Self::setup_graphics(world, shader, texture, mesh);
        GameController {
            geometry_pass_id: geometry_pass,
            aabb_pass_id: aabb_pass,
        }
        .attach_to_ecs(world);
    }
}

// ---------- Monitoring / Logging ----------

fn main_loop_profile_handler(r: Receiver<MainLoopMonitoring>) {
    info!(
        "Main loop: {:.1} tps ({:.1}%)",
        r.event.tps.average(),
        r.event.load.average() * 100.0
    );
}

fn renderer_profile_handler(r: Receiver<RendererMonitoring>) {
    info!(
        "Renderer: {:.1} FPS | render/view (ms): {:.1}/{:.1}",
        r.event.fps.average(),
        r.event.render.average().as_millis(),
        r.event.view.average().as_millis(),
    );
}

// ---------- Input handling ----------

fn escape_handler(r: Receiver<InputEvent>, mut s: Sender<StopEventLoop>) {
    if let InputEvent::KeyRelease(KeyCode::Escape) = r.event {
        info!("Escape key released, stopping the event loop");
        s.send(StopEventLoop);
    }
}

/// Handle window resize + a few quality‑of‑life keys:
///  - SPACE : toggle rotation pause
///  - R     : reset rotations
///  - +/-   : change spin speed
///  - F1    : print quick help
fn events_handler(
    ie: Receiver<InputEvent>,
    gc: Single<&mut GameController>,
    mut s: Sender<RenderPassEvent<CustomPassEvent>>,
    mut spins: Fetcher<&mut Spin>,
    mut rotations: Fetcher<&mut Rotation>,
) {
    match ie.event {
        InputEvent::Resize { width, height } => {
            info!("Window resized to {}x{}", width, height);
            s.send(RenderPassEvent::new(
                gc.geometry_pass_id,
                CustomPassEvent::UpdateWindowSize(*width, *height),
            ));
        }
        InputEvent::KeyRelease(key) => {
            let key_name = format!("{:?}", key);
            match key_name.as_str() {
                "Space" => {
                    let mut toggled = false;
                    for sp in &mut spins {
                        sp.paused = !sp.paused;
                        toggled = true;
                    }
                    if toggled {
                        info!("Spin {}", if spins.into_iter().next().is_some() { "toggled" } else { "state changed" });
                    } else {
                        warn!("No Spin components to toggle.");
                    }
                }
                "R" => {
                    for rot in &mut rotations {
                        rot.0 = Quat::IDENTITY;
                    }
                    info!("Rotations reset.");
                }
                "Plus" | "Equals" => {
                    for sp in &mut spins {
                        sp.speed_x *= 1.2;
                        sp.speed_y *= 1.2;
                    }
                    info!("Spin speed increased x1.2");
                }
                "Minus" | "Subtract" => {
                    for sp in &mut spins {
                        sp.speed_x *= 0.8;
                        sp.speed_y *= 0.8;
                    }
                    info!("Spin speed decreased x0.8");
                }
                "F1" => {
                    info!("Keys: [ESC]=quit  [F1]=help  [SPACE]=pause spin  [R]=reset  [+/-]=spin speed");
                }
                _ => {}
            }
        }
        _ => {}
    }
}

// ---------- Asset events ----------

fn assets_failed_handler(r: Receiver<AssetHubEvent>, mut stopper: Sender<StopEventLoop>) {
    match r.event {
        AssetHubEvent::LoadFailed(id, kind, err) => {
            error!("Asset load failed: {:?} ({:?}) -> {}", id, kind, err);
            error!("Aborting due to asset load failure");
            stopper.send(StopEventLoop);
        }
        AssetHubEvent::AllAssetsFreed => {
            info!("All assets have been freed");
            stopper.send(StopEventLoop);
        }
        _ => {}
    }
}

fn assets_loaded_handler(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    gc: Single<&GameController>,
    mut rpe: Sender<RenderPassEvent<CustomPassEvent>>,
) {
    if let AssetHubEvent::AllAssetsLoaded = r.event {
        info!("All assets loaded.");
        // Update the geometry shader after load
        match hub.get_typed::<ShaderProgram>(AssetID::from("geometry")) {
            Ok(shader) => {
                rpe.send(RenderPassEvent::new(
                    gc.geometry_pass_id,
                    CustomPassEvent::UpdateShader(shader),
                ));
                info!("Geometry shader bound to geometry pass.");
            }
            Err(e) => error!("Missing 'geometry' shader: {}", e),
        }
        // Optional: you could dispatch more pass updates here (textures, options, etc.)
    }
}

/// Spawn one or a grid of cubes upon assets loaded
fn assets_spawn(
    r: Receiver<AssetHubEvent>,
    hub: Single<&mut AssetHub>,
    mut spawn: Sender<(
        Spawn,
        Insert<Position>,
        Insert<Scale>,
        Insert<RenderableMesh>,
        Insert<Rotation>,
        Insert<Spin>,
    )>,
) {
    if let AssetHubEvent::AllAssetsLoaded = r.event {
        let grid_n = env_parse::<u32>("DAWN_CUBE_GRID", 1).clamp(1, 32);
        let spacing = env_parse::<f32>("DAWN_CUBE_SPACING", 2.5);
        let scale = env_parse::<f32>("DAWN_CUBE_SCALE", 1.0).max(0.01);
        let z = env_parse::<f32>("DAWN_CUBE_Z", -8.0);

        let mesh = match hub.get_typed::<Mesh>(AssetID::from("cube")) {
            Ok(m) => m,
            Err(e) => {
                error!("Missing 'cube' mesh: {}", e);
                return;
            }
        };

        let center = (grid_n as f32 - 1.0) * 0.5 * spacing;
        let total = grid_n * grid_n;
        info!(
            "Spawning {} cube(s) in a {}x{} grid (spacing={}, scale={}, z={})",
            total, grid_n, grid_n, spacing, scale, z
        );

        for ix in 0..grid_n {
            for iy in 0..grid_n {
                let id = spawn.spawn();
                let x = ix as f32 * spacing - center;
                let y = iy as f32 * spacing - center;
                spawn.insert(id, RenderableMesh(mesh));
                spawn.insert(id, Rotation(Quat::IDENTITY));
                spawn.insert(id, Scale(Vec3::splat(scale)));
                spawn.insert(id, Position(Vec3::new(x, y, z)));
                // Vary spin slightly for visual interest
                let sx = 0.3 + (ix as f32) * 0.05;
                let sy = 0.6 + (iy as f32) * 0.05;
                spawn.insert(
                    id,
                    Spin {
                        speed_x: sx,
                        speed_y: sy,
                        paused: false,
                    },
                );
            }
        }
    }
}

// ---------- Simulation ----------

/// Use per‑entity `Spin` to update `Rotation`. If no `Spin` is present, nothing happens.
fn rotate_handler(t: Receiver<Tick>, mut q: Fetcher<(&mut Rotation, &Spin)>) {
    let dt = t.event.delta;
    for (rot, spin) in q {
        if !spin.paused {
            let rx = Quat::from_rotation_x(spin.speed_x * dt);
            let ry = Quat::from_rotation_y(spin.speed_y * dt);
            rot.0 = rot.0 * ry * rx;
        }
    }
}

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let mut world = World::new();
    GameController::setup(&mut world);

    // Core monitoring handlers
    world.add_handler(main_loop_profile_handler);
    world.add_handler(renderer_profile_handler);
    world.add_handler(escape_handler);

    // Asset handlers
    world.add_handler(assets_failed_handler);
    world.add_handler(assets_loaded_handler);
    world.add_handler(assets_spawn);

    // Input + render events
    world.add_handler(events_handler);

    // Simulation
    world.add_handler(rotate_handler);

    let target_tps = detect_refresh_rate();
    run_loop_with_monitoring(&mut world, target_tps);
}
