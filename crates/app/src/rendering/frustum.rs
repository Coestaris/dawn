// Taken from:
// [ https://gist.github.com/griffi-gh/a6ed5ed3bc7e7ac8e29974502abafb40 ]
// That is ported from C++. Used as a reference:
//   [ https://github.com/Beastwick18/gltest/blob/main/src/renderer/Frustum.cpp ]
//   - original code:
//     [ https://gist.github.com/podgorskiy/e698d18879588ada9014768e3e82a644 ]
//     - which uses cube vs frustum intersection code from:
//       [ http://iquilezles.org/www/articles/frustumcorrect/frustumcorrect.htm ]

use glam::*;

#[repr(usize)]
enum FrustumPlane {
    Left,
    Right,
    Bottom,
    Top,
    Near,
    Far,
}

const PLANE_COUNT: usize = 6;
const PLANE_COMBINATIONS: usize = PLANE_COUNT * (PLANE_COUNT - 1) / 2;
const POINT_COUNT: usize = 8;

#[derive(Default)]
pub struct FrustumCulling {
    planes: [Vec4; PLANE_COUNT],
    points: [Vec3A; POINT_COUNT],
    perspective: Mat4,
    view: Mat4,
}

fn vec4_to_vec3a(v: Vec4) -> Vec3A {
    Vec3A::new(v.x, v.y, v.z)
}

impl FrustumCulling {
    pub fn new() -> Self {
        let mut frustum = FrustumCulling::default();
        frustum.update_state();
        frustum
    }

    pub fn set_perspective(&mut self, perspective: Mat4) {
        self.perspective = perspective;
        self.update_state();
    }

    pub fn set_view(&mut self, view: Mat4) {
        self.view = view;
        self.update_state();
    }

    fn update_state(&mut self) {
        //compute transposed view-projection matrix
        let mat = (self.perspective * self.view).transpose();

        // compute planes
        let mut planes = [Vec4::default(); PLANE_COUNT];
        planes[FrustumPlane::Left as usize] = mat.w_axis + mat.x_axis;
        planes[FrustumPlane::Right as usize] = mat.w_axis - mat.x_axis;
        planes[FrustumPlane::Bottom as usize] = mat.w_axis + mat.y_axis;
        planes[FrustumPlane::Top as usize] = mat.w_axis - mat.y_axis;
        planes[FrustumPlane::Near as usize] = mat.w_axis + mat.z_axis;
        planes[FrustumPlane::Far as usize] = mat.w_axis - mat.z_axis;

        //compute crosses
        let crosses = [
            vec4_to_vec3a(planes[FrustumPlane::Left as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Right as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Left as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Bottom as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Left as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Top as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Left as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Near as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Left as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Far as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Right as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Bottom as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Right as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Top as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Right as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Near as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Right as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Far as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Bottom as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Top as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Bottom as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Near as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Bottom as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Far as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Top as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Near as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Top as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Far as usize])),
            vec4_to_vec3a(planes[FrustumPlane::Near as usize])
                .cross(vec4_to_vec3a(planes[FrustumPlane::Far as usize])),
        ];

        //compute points
        let points = [
            intersection::<
                { FrustumPlane::Left as usize },
                { FrustumPlane::Bottom as usize },
                { FrustumPlane::Near as usize },
            >(&planes, &crosses),
            intersection::<
                { FrustumPlane::Left as usize },
                { FrustumPlane::Top as usize },
                { FrustumPlane::Near as usize },
            >(&planes, &crosses),
            intersection::<
                { FrustumPlane::Right as usize },
                { FrustumPlane::Bottom as usize },
                { FrustumPlane::Near as usize },
            >(&planes, &crosses),
            intersection::<
                { FrustumPlane::Right as usize },
                { FrustumPlane::Top as usize },
                { FrustumPlane::Near as usize },
            >(&planes, &crosses),
            intersection::<
                { FrustumPlane::Left as usize },
                { FrustumPlane::Bottom as usize },
                { FrustumPlane::Far as usize },
            >(&planes, &crosses),
            intersection::<
                { FrustumPlane::Left as usize },
                { FrustumPlane::Top as usize },
                { FrustumPlane::Far as usize },
            >(&planes, &crosses),
            intersection::<
                { FrustumPlane::Right as usize },
                { FrustumPlane::Bottom as usize },
                { FrustumPlane::Far as usize },
            >(&planes, &crosses),
            intersection::<
                { FrustumPlane::Right as usize },
                { FrustumPlane::Top as usize },
                { FrustumPlane::Far as usize },
            >(&planes, &crosses),
        ];

        self.planes = planes;
        self.points = points;
    }

    #[inline]
    pub fn obb_to_aabb(min: Vec3, max: Vec3, model: Mat4) -> (Vec3, Vec3) {
        let c = (min + max) * 0.5;
        let e = (max - min) * 0.5;

        let ax = model.x_axis.truncate();
        let ay = model.y_axis.truncate();
        let az = model.z_axis.truncate();
        let t = model.w_axis.truncate();

        let c_world = ax * c.x + ay * c.y + az * c.z + t;
        let e_world = ax.abs() * e.x + ay.abs() * e.y + az.abs() * e.z;

        (c_world - e_world, c_world + e_world)
    }

    pub fn is_visible(&self, minp: Vec3, maxp: Vec3, model: Mat4) -> bool {
        let (minp, maxp) = Self::obb_to_aabb(minp, maxp, model);
        self.is_box_visible(minp, maxp)
    }

    pub fn is_box_visible(&self, minp: Vec3, maxp: Vec3) -> bool {
        // check box outside/inside of frustum
        for plane in self.planes {
            if (plane.dot(vec4(minp.x, minp.y, minp.z, 1.)) < 0.)
                && (plane.dot(vec4(maxp.x, minp.y, minp.z, 1.)) < 0.)
                && (plane.dot(vec4(minp.x, maxp.y, minp.z, 1.)) < 0.)
                && (plane.dot(vec4(maxp.x, maxp.y, minp.z, 1.)) < 0.)
                && (plane.dot(vec4(minp.x, minp.y, maxp.z, 1.)) < 0.)
                && (plane.dot(vec4(maxp.x, minp.y, maxp.z, 1.)) < 0.)
                && (plane.dot(vec4(minp.x, maxp.y, maxp.z, 1.)) < 0.)
                && (plane.dot(vec4(maxp.x, maxp.y, maxp.z, 1.)) < 0.)
            {
                return false;
            }
        }

        // check frustum outside/inside box
        if self.points.iter().all(|point| point.x > maxp.x) {
            return false;
        }
        if self.points.iter().all(|point| point.x < minp.x) {
            return false;
        }
        if self.points.iter().all(|point| point.y > maxp.y) {
            return false;
        }
        if self.points.iter().all(|point| point.y < minp.y) {
            return false;
        }
        if self.points.iter().all(|point| point.z > maxp.z) {
            return false;
        }
        if self.points.iter().all(|point| point.z < minp.z) {
            return false;
        }

        true
    }
}

const fn ij2k<const I: usize, const J: usize>() -> usize {
    I * (9 - I) / 2 + J - 1
}
fn intersection<const A: usize, const B: usize, const C: usize>(
    planes: &[Vec4; PLANE_COUNT],
    crosses: &[Vec3A; PLANE_COMBINATIONS],
) -> Vec3A {
    let d = vec4_to_vec3a(planes[A]).dot(crosses[ij2k::<B, C>()]);
    let res = Mat3A::from_cols(
        crosses[ij2k::<B, C>()],
        -crosses[ij2k::<A, C>()],
        crosses[ij2k::<A, B>()],
    ) * vec3a(planes[A].w, planes[B].w, planes[C].w);
    res * (-1. / d)
}
