use glam::{Mat4, Vec3, Vec4};

use crate::xnb::asset::model::{BoundingBox, BoundingSphere};

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub pitch_radians: f32,
    pub yaw_radians: f32,
    pub fov_y_radians: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub const UP: Vec3 = Vec3::Y;

    pub fn look_at(&mut self, target: Vec3) {
        let forward = (target - self.position).normalize();
        self.yaw_radians = forward.x.atan2(forward.z);
        self.pitch_radians = forward.y.asin();
    }

    pub fn forward_right_up(&self) -> (Vec3, Vec3, Vec3) {
        let up = Self::UP;
        let forward_x = self.yaw_radians.sin() * self.pitch_radians.cos();
        let forward_y = self.pitch_radians.sin();
        let forward_z = self.yaw_radians.cos() * self.pitch_radians.cos();
        let forward = Vec3::new(forward_x, forward_y, forward_z).normalize();
        let right = forward.cross(up);
        (forward, right, up)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward_right_up().0, Self::UP)
    }
}

pub struct Frustum {
    pub near: Plane,
    pub far: Plane,
    pub bottom: Plane,
    pub top: Plane,
    pub left: Plane,
    pub right: Plane,
}

pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Frustum {
    pub fn new(view_proj: Mat4) -> Self {
        let m = view_proj.to_cols_array_2d();

        let near = Vec4::new(
            m[0][3] + m[0][2],
            m[1][3] + m[1][2],
            m[2][3] + m[2][2],
            m[3][3] + m[3][2],
        );
        let far = Vec4::new(
            m[0][3] - m[0][2],
            m[1][3] - m[1][2],
            m[2][3] - m[2][2],
            m[3][3] - m[3][2],
        );
        let bottom = Vec4::new(
            m[0][3] + m[0][1],
            m[1][3] + m[1][1],
            m[2][3] + m[2][1],
            m[3][3] + m[3][1],
        );
        let top = Vec4::new(
            m[0][3] - m[0][1],
            m[1][3] - m[1][1],
            m[2][3] - m[2][1],
            m[3][3] - m[3][1],
        );
        let left = Vec4::new(
            m[0][3] + m[0][0],
            m[1][3] + m[1][0],
            m[2][3] + m[2][0],
            m[3][3] + m[3][0],
        );
        let right = Vec4::new(
            m[0][3] - m[0][0],
            m[1][3] - m[1][0],
            m[2][3] - m[2][0],
            m[3][3] - m[3][0],
        );

        let [near, far, bottom, top, left, right] =
            [near, far, bottom, top, left, right].map(|p| {
                let normal = Vec3::new(p.x, p.y, p.z);
                let inv_len = 1.0 / normal.length();
                Plane {
                    normal: normal * inv_len,
                    distance: p.w * inv_len,
                }
            });

        Frustum {
            near,
            far,
            bottom,
            top,
            left,
            right,
        }
    }

    pub fn planes(&self) -> [&Plane; 6] {
        [
            &self.near,
            &self.far,
            &self.bottom,
            &self.top,
            &self.left,
            &self.right,
        ]
    }

    pub fn test_sphere(&self, sphere: &BoundingSphere) -> bool {
        todo!()
    }

    pub fn test_aabb(&self, aabb: &BoundingBox) -> bool {
        for p in self.planes() {
            let vertex = Vec3::new(
                if p.normal.x >= 0.0 {
                    aabb.max.x
                } else {
                    aabb.min.x
                },
                if p.normal.y >= 0.0 {
                    aabb.max.y
                } else {
                    aabb.min.y
                },
                if p.normal.z >= 0.0 {
                    aabb.max.z
                } else {
                    aabb.min.z
                },
            );

            if p.normal.dot(vertex) + p.distance < 0.0 {
                return false;
            }
        }

        true
    }
}
