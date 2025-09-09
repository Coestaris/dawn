use dawn_assets::ir::texture::{IRPixelFormat, IRTextureFilter, IRTextureWrap};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::renderable::RenderablePointLight;
use glam::{UVec4, Vec4};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LightsHeaderCPU {
    // magic 'LITE' (0x4C495445), version, num_lights, reserved
    pub meta0: [u32; 4],
}

impl LightsHeaderCPU {
    pub fn new(num_lights: u32) -> Self {
        Self {
            meta0: [0x4C495445, 1, num_lights, 0],
        }
    }

    pub fn as_uvec4(&self) -> UVec4 {
        UVec4::from_array(self.meta0)
    }
}

const LIGHT_KIND_DIRECTIONAL: u8 = 0;
const LIGHT_KIND_POINT: u8 = 1;
const LIGHT_KIND_AREA_RECT: u8 = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LightPackedCPU {
    // x=kind, y=flags, z=reserved, w=float bits of intensity
    pub kind_flags_intensity: [u32; 4],
    // rgb=color, a=unused
    pub color_rgba: [f32; 4],
    // sun: dir; point: pos.xyz, w=radius
    pub v0: [f32; 4],
    // area: normal/halfHeight; others: reserved
    pub v1: [f32; 4],
    // rough, metallic, falloff(0 phys / 1 lin), shadow
    pub brdf: [f32; 4],
}

pub struct PackedLights {
    gl: &'static glow::Context,
    capacity_texels: i32,
    pub texture: Texture,
    vec: Vec<f32>,
}

impl PackedLights {
    pub fn new(gl: &'static glow::Context) -> Option<Self> {
        let texture = Texture::new2d(gl).ok()?;

        Texture::bind(gl, TextureBind::Texture2D, &texture, 0);
        texture.set_mag_filter(IRTextureFilter::Nearest).ok()?;
        texture.set_min_filter(IRTextureFilter::Nearest).ok()?;
        texture.set_wrap_r(IRTextureWrap::ClampToEdge).ok()?;
        texture.set_wrap_s(IRTextureWrap::ClampToEdge).ok()?;
        texture.set_wrap_t(IRTextureWrap::ClampToEdge).ok()?;
        Texture::unbind(gl, TextureBind::Texture2D, 0);

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

    fn push_packed(&mut self, l: &LightPackedCPU) {
        for &u in &l.kind_flags_intensity {
            self.vec.push(f32::from_bits(u));
        }
        self.vec.extend_from_slice(&l.color_rgba);
        self.vec.extend_from_slice(&l.v0);
        self.vec.extend_from_slice(&l.v1);
        self.vec.extend_from_slice(&l.brdf);
    }

    pub fn push_point_light(&mut self, l: &RenderablePointLight, view_mat: &glam::Mat4) {
        let mut packed = LightPackedCPU::default();
        packed.kind_flags_intensity[0] = LIGHT_KIND_POINT as u32;
        packed.kind_flags_intensity[1] = 0;
        packed.kind_flags_intensity[2] = 0;
        packed.kind_flags_intensity[3] = l.intensity.to_bits();
        packed.color_rgba[0] = l.color.x;
        packed.color_rgba[1] = l.color.y;
        packed.color_rgba[2] = l.color.z;
        let view_pos = view_mat * l.position.extend(1.0);
        packed.v0[0] = view_pos.x;
        packed.v0[1] = view_pos.y;
        packed.v0[2] = view_pos.z;
        packed.v0[3] = l.range;
        packed.brdf = [0.0, 0.0, 0.0, 0.0];
        self.push_packed(&packed);
    }

    pub fn upload(&mut self) {
        Texture::bind(self.gl, TextureBind::Texture2D, &self.texture, 0);

        let needed_texels = self.vec.len() as i32;
        if needed_texels > self.capacity_texels {
            self.capacity_texels = (needed_texels * 2).max(16);
            self.texture
                .feed_image(
                    0,
                    self.capacity_texels as usize,
                    1,
                    false,
                    IRPixelFormat::RGBA32F,
                    None,
                )
                .ok();
        }

        self.texture
            .feed_image(
                0,
                needed_texels as usize,
                1,
                false,
                IRPixelFormat::RGBA32F,
                Some(bytemuck::cast_slice(&self.vec)),
            )
            .ok();
        Texture::unbind(self.gl, TextureBind::Texture2D, 0);
    }
}
