use dawn_graphics::gl::raii::ubo::UBO;
use std::sync::Arc;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct CameraUBOPayload {
    pub in_view: [[f32; 4]; 4],
    pub in_projection: [[f32; 4]; 4],
    pub in_inv_proj: [[f32; 4]; 4],
    pub in_inv_view: [[f32; 4]; 4],
    pub in_viewport: [f32; 2],    // w, h
    pub in_clip_planes: [f32; 2], // near, far
}

impl CameraUBOPayload {
    pub fn new_inner(
        view: [[f32; 4]; 4],
        proj: [[f32; 4]; 4],
        inv_proj: [[f32; 4]; 4],
        inv_view: [[f32; 4]; 4],
        viewport_wh: [f32; 2],
        planes: [f32; 2],
    ) -> Self {
        Self {
            in_view: view,
            in_projection: proj,
            in_inv_proj: inv_proj,
            in_inv_view: inv_view,
            in_viewport: viewport_wh,
            in_clip_planes: planes,
        }
    }

    pub fn from_glam(
        view: glam::Mat4,
        proj: glam::Mat4,
        viewport_wh: [f32; 2],
        inv_view: Option<glam::Mat4>,
        inv_proj: Option<glam::Mat4>,
        near: f32,
        far: f32,
    ) -> Self {
        let inv_v = inv_view.unwrap_or_else(|| view.inverse());
        let inv_p = inv_proj.unwrap_or_else(|| proj.inverse());
        Self::new_inner(
            view.to_cols_array_2d(),
            proj.to_cols_array_2d(),
            inv_p.to_cols_array_2d(),
            inv_v.to_cols_array_2d(),
            viewport_wh,
            [near, far],
        )
    }
}

pub struct CameraUBO {
    gl: Arc<glow::Context>,
    pub ubo: UBO,
    pub payload: CameraUBOPayload,
    pub binding: usize,
}

impl CameraUBO {
    pub(crate) fn new(gl: Arc<glow::Context>, binding: usize) -> Self {
        let ubo = UBO::new(gl.clone(), Some(size_of::<CameraUBOPayload>())).unwrap();
        UBO::bind(&gl, &ubo);
        ubo.bind_base(binding as u32);
        UBO::unbind(&gl);

        CameraUBO {
            gl,
            ubo,
            payload: CameraUBOPayload::from_glam(
                glam::Mat4::IDENTITY,
                glam::Mat4::IDENTITY,
                [1.0, 1.0],
                None,
                None,
                0.1,
                100.0,
            ),
            binding,
        }
    }

    pub fn set_viewport(&mut self, w: f32, h: f32) {
        self.payload.in_viewport = [w, h];
    }

    pub fn set_view(&mut self, view: glam::Mat4) {
        self.payload.in_view = view.to_cols_array_2d();
        self.payload.in_inv_view = view.inverse().to_cols_array_2d();
    }

    pub fn set_perspective(&mut self, proj: glam::Mat4, near: f32, far: f32) {
        self.payload.in_projection = proj.to_cols_array_2d();
        self.payload.in_inv_proj = proj.inverse().to_cols_array_2d();
        self.payload.in_clip_planes = [near, far];
    }

    pub fn upload(&self) {
        UBO::bind(&self.gl, &self.ubo);
        self.ubo.feed(unsafe {
            std::slice::from_raw_parts(
                &self.payload as *const CameraUBOPayload as *const u8,
                size_of::<CameraUBOPayload>(),
            )
        });
        UBO::unbind(&self.gl);
    }
}
