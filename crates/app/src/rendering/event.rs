use bitflags::bitflags;
use dawn_assets::{AssetID, TypedAsset};
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::Texture2D;
use glam::{Mat4, UVec2};

#[derive(Debug, Clone)]
pub enum LightTextureType {
    SunLight,
    PointLight,
}

#[derive(Debug, Clone)]
pub enum RenderingEvent {
    // Generic events
    DropAllAssets,
    UpdateShader(AssetID, TypedAsset<Program>),
    ViewUpdated(Mat4),
    PerspectiveProjectionUpdated(Mat4, f32, f32), /* proj, near, far */
    OrthographicProjectionUpdated(Mat4),
    ViewportResized(UVec2),

    // Specific events can be added here
    SetLightTexture(LightTextureType, TypedAsset<Texture2D>),
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct RenderingEventMask: u32 {
        const DROP_ALL_ASSETS = 1;
        const UPDATE_SHADER = 1 << 1;
        const VIEW_UPDATED = 1 << 2;
        const PERSP_PROJECTION_UPDATED = 1 << 3;
        const ORTHO_PROJECTION_UPDATED = 1 << 4;
        const VIEWPORT_RESIZED = 1 << 5;

        const SET_LIGHT_TEXTURE = 1 << 10;
    }
}
