use dawn_assets::ir::texture::{IRPixelFormat, IRTextureFilter, IRTextureWrap};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::renderable::{RenderablePointLight, RenderableSunLight};
use glam::UVec4;
use std::sync::Arc;

#[repr(C)]
#[repr(packed)]
#[derive(Clone, Copy, Default)]
pub struct LightsHeaderPayload {
    // magic 'LITE' (0x4C495445), version, num_lights, reserved
    pub meta0: [u32; 4],
}

impl LightsHeaderPayload {
    pub fn new(num_lights: u32) -> Self {
        Self {
            meta0: [0x4C495445, 1, num_lights, 0],
        }
    }

    pub fn as_uvec4(&self) -> UVec4 {
        UVec4::from_array(self.meta0)
    }
}

const LIGHT_KIND_SUN: u32 = 1;
const LIGHT_KIND_SPOT: u32 = 2;
const LIGHT_KIND_POINT: u32 = 3;
const LIGHT_KIND_AREA_RECT: u32 = 4;

#[repr(C)]
#[repr(packed)]
#[derive(Clone, Copy, Default)]
struct LightPackedPayload {
    pub kind: u32,
    pub flags: u32,
    pub reserved: u32,
    pub intensity: f32,

    // sun: rgb
    // spot: rgb, a=outer angle (cosine)
    // point: rgb, a=unused
    pub color_rgba: [f32; 4],

    // sun: dir.xyz, w=ambient
    // spot: pos.xyz, w=range
    // point: pos.xyz, w=radius
    pub v0: [f32; 4],

    // sun: unused
    // spot: dir.xyz, w=inner angle (cosine)
    // point: unused
    pub v1: [f32; 4],

    pub rough: f32,
    pub metallic: f32,
    // sun: unused
    // spot: linear falloff (1.0 = linear, 0.0 = quadratic)
    // point: linear falloff (1.0 = linear, 0.0 = quadratic)
    pub falloff: f32,
    // sun: shadow (1.0 = yes, 0.0 = no)
    // spot: shadow (1.0 = yes, 0.0 = no)
    // point: shadow (1.0 = yes, 0.0 = no)
    pub shadow: f32,
}

pub struct PackedLights {
    gl: Arc<glow::Context>,
    capacity_texels: i32,
    pub texture: Texture,
    vec: Vec<u32>,
}

impl PackedLights {
    pub fn new(gl: Arc<glow::Context>) -> Option<Self> {
        let texture = Texture::new2d(gl.clone()).ok()?;

        Texture::bind(&gl, TextureBind::Texture2D, &texture, 0);
        texture.set_mag_filter(IRTextureFilter::Nearest).ok()?;
        texture.set_min_filter(IRTextureFilter::Nearest).ok()?;
        texture.set_wrap_r(IRTextureWrap::ClampToEdge).ok()?;
        texture.set_wrap_s(IRTextureWrap::ClampToEdge).ok()?;
        texture.set_wrap_t(IRTextureWrap::ClampToEdge).ok()?;
        Texture::unbind(&gl, TextureBind::Texture2D, 0);

        Some(Self {
            gl,
            texture,
            capacity_texels: 0,
            vec: Vec::new(),
        })
    }

    pub fn clear(&mut self) {
        self.vec.clear();
    }

    fn push_packed(&mut self, l: &LightPackedPayload) {
        self.vec.push(l.kind);
        self.vec.push(l.flags);
        self.vec.push(l.reserved);
        self.vec.push(l.intensity.to_bits());
        for c in l.color_rgba {
            self.vec.push(c.to_bits());
        }
        for v in l.v0 {
            self.vec.push(v.to_bits());
        }
        for v in l.v1 {
            self.vec.push(v.to_bits());
        }
        self.vec.push(l.rough.to_bits());
        self.vec.push(l.metallic.to_bits());
        self.vec.push(l.falloff.to_bits());
        self.vec.push(l.shadow.to_bits());
    }

    pub fn push_point_light(&mut self, l: &RenderablePointLight, view_mat: &glam::Mat4) {
        let mut packed = LightPackedPayload::default();
        packed.kind = LIGHT_KIND_POINT;
        packed.flags = 0;
        packed.reserved = 0;
        packed.intensity = l.intensity;
        packed.color_rgba[0] = l.color.x;
        packed.color_rgba[1] = l.color.y;
        packed.color_rgba[2] = l.color.z;
        let view_pos = view_mat * l.position.extend(1.0);
        packed.v0[0] = view_pos.x;
        packed.v0[1] = view_pos.y;
        packed.v0[2] = view_pos.z;
        packed.v0[3] = l.range;
        packed.v1 = [0.0; 4];
        packed.rough = 0.0;
        packed.metallic = 0.0;
        packed.falloff = if l.linear_falloff { 1.0 } else { 0.0 };
        packed.shadow = 0.0;
        self.push_packed(&packed);
    }

    pub fn push_sun_light(&mut self, l: &RenderableSunLight, view_mat: &glam::Mat4) {
        let mut packed = LightPackedPayload::default();
        packed.kind = LIGHT_KIND_SUN;
        packed.flags = 0;
        packed.reserved = 0;
        packed.intensity = l.intensity;
        packed.color_rgba[0] = l.color.x;
        packed.color_rgba[1] = l.color.y;
        packed.color_rgba[2] = l.color.z;
        let view_dir = view_mat * l.direction.extend(0.0);
        packed.v0[0] = view_dir.x;
        packed.v0[1] = view_dir.y;
        packed.v0[2] = view_dir.z;
        packed.v0[3] = l.ambient;
        packed.v1 = [0.0; 4];
        packed.rough = 0.0;
        packed.metallic = 0.0;
        packed.falloff = 0.0;
        packed.shadow = 0.0;
        self.push_packed(&packed);
    }

    pub fn upload(&mut self) {
        Texture::bind(&self.gl, TextureBind::Texture2D, &self.texture, 0);

        let needed_texels = self.vec.len() as i32;
        if needed_texels > self.capacity_texels {
            self.capacity_texels = (needed_texels * 2).max(16);
            self.texture
                .feed_2d::<()>(
                    0,
                    self.capacity_texels as usize,
                    1,
                    false,
                    IRPixelFormat::RGBA32UI,
                    None,
                )
                .ok();
        }

        self.texture
            .feed_2d(
                0,
                needed_texels as usize,
                1,
                false,
                IRPixelFormat::RGBA32UI,
                Some(self.vec.as_slice()),
            )
            .ok();
        Texture::unbind(&self.gl, TextureBind::Texture2D, 0);
    }
}
