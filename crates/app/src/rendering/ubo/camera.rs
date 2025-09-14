use dawn_graphics::gl::raii::ubo::UBO;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct CameraUBOPayload {
    pub in_view: [[f32; 4]; 4],
    pub in_projection: [[f32; 4]; 4],
    pub in_inv_proj: [[f32; 4]; 4],
    pub in_inv_view: [[f32; 4]; 4],
    pub in_viewport: [f32; 2], // w, h
    pub _pad_cam: [f32; 2],    // std140: pad to 16 bytes
}

impl CameraUBOPayload {
    pub fn new_inner(
        view: [[f32; 4]; 4],
        proj: [[f32; 4]; 4],
        inv_proj: [[f32; 4]; 4],
        inv_view: [[f32; 4]; 4],
        viewport_wh: [f32; 2],
    ) -> Self {
        Self {
            in_view: view,
            in_projection: proj,
            in_inv_proj: inv_proj,
            in_inv_view: inv_view,
            in_viewport: viewport_wh,
            _pad_cam: [0.0; 2],
        }
    }

    pub fn from_glam(
        view: glam::Mat4,
        proj: glam::Mat4,
        viewport_wh: [f32; 2],
        // если None — посчитаем внутри
        inv_view: Option<glam::Mat4>,
        inv_proj: Option<glam::Mat4>,
    ) -> Self {
        let inv_v = inv_view.unwrap_or_else(|| view.inverse());
        let inv_p = inv_proj.unwrap_or_else(|| proj.inverse());
        Self::new_inner(
            view.to_cols_array_2d(),
            proj.to_cols_array_2d(),
            inv_p.to_cols_array_2d(),
            inv_v.to_cols_array_2d(),
            viewport_wh,
        )
    }
}

pub struct CameraUBO {
    gl: &'static glow::Context,
    pub ubo: UBO,
    pub payload: CameraUBOPayload,
    pub binding: usize,
}

impl CameraUBO {
    pub(crate) fn new(gl: &'static glow::Context, binding: usize) -> Self {
        let ubo = UBO::new(gl, Some(size_of::<CameraUBOPayload>())).unwrap();
        UBO::bind(gl, &ubo);
        ubo.bind_base(binding as u32);
        UBO::unbind(gl);

        CameraUBO {
            gl,
            ubo,
            payload: CameraUBOPayload::from_glam(
                glam::Mat4::IDENTITY,
                glam::Mat4::IDENTITY,
                [1.0, 1.0],
                None,
                None,
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

    pub fn set_perspective(&mut self, proj: glam::Mat4) {
        self.payload.in_projection = proj.to_cols_array_2d();
        self.payload.in_inv_proj = proj.inverse().to_cols_array_2d();
    }

    pub fn upload(&self) {
        UBO::bind(self.gl, &self.ubo);
        self.ubo.feed(unsafe {
            std::slice::from_raw_parts(
                &self.payload as *const CameraUBOPayload as *const u8,
                size_of::<CameraUBOPayload>(),
            )
        });
        UBO::unbind(self.gl);
    }
}
