use crate::input::YawPitch;
use nalgebra::{Matrix4, Perspective3, Vector3, Vector4};
use voxel_rs_common::world::{ChunkPos, CHUNK_SIZE};

/// A plane in the 3d space, i.e. all points (x, y, z) such that ax + by + cz = d.
#[derive(Debug, Copy, Clone)]
pub struct Plane {
    /// (a, b, c)
    pub normal: Vector3<f64>,
    /// d
    pub d: f64,
}

impl Plane {
    pub fn dist(&self, point: Vector3<f64>) -> f64 {
        (self.normal.dot(&point) + self.d) / self.normal.norm()
    }
}

const FOV: f64 = 90.0f64 * 2.0 * std::f64::consts::PI / 360.0;

/// The player's frustum
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    /// Position of the camera
    pub position: Vector3<f64>,
    /// Yaw in degrees
    pub yaw: f64,
    /// Yaw in degrees
    pub pitch: f64,
}

impl Frustum {
    /// Create a new frustum. This function should be called each frame.
    pub fn new(position: Vector3<f64>, yaw_pitch: YawPitch) -> Frustum {
        Self {
            position,
            yaw: yaw_pitch.yaw,
            pitch: yaw_pitch.pitch,
        }
    }

    /// Get the view/projection matrix associated with this frustum
    pub fn get_view_projection(&self, aspect_ratio: f64) -> Matrix4<f64> {
        let proj = Perspective3::new(aspect_ratio, FOV, 0.1, 3000.0);
        proj.as_matrix() * self.get_view_matrix()
    }

    pub fn get_view_matrix(&self) -> Matrix4<f64> {
        let rotation = Matrix4::from_euler_angles(-self.pitch.to_radians(), 0.0, 0.0)
            * Matrix4::from_euler_angles(0.0, -self.yaw.to_radians(), 0.0);
        let translation = Matrix4::new_translation(&-self.position);
        rotation * translation
    }

    pub fn get_planes(&self, aspect_ratio: f64) -> [[Plane; 2]; 3] {
        let (fovy, znear, zfar) = (FOV, 0.1, 3000.0);
        let t = (fovy / 2.0).tan();
        let h_near = t * 2.0 * znear;
        let w_near = h_near * aspect_ratio;
        let up = Vector3::new(0.0, 1.0, 0.0);
        let right = Vector3::new(1.0, 0.0, 0.0);
        let near_center = Vector3::new(0.0, 0.0, -znear);
        let near_right = near_center + Vector3::new(w_near * 0.5, 0.0, 0.0);
        let near_left = near_center - Vector3::new(w_near * 0.5, 0.0, 0.0);
        let near_top = near_center + Vector3::new(h_near * 0.5, 0.0, 0.0);
        let near_bottom = near_center - Vector3::new(h_near * 0.5, 0.0, 0.0);

        fn get_side_plane(point: Vector3<f64>, other_vector: Vector3<f64>) -> Plane {
            let normal = point.cross(&other_vector);
            Plane {
                normal,
                d: -normal.dot(&point) / normal.norm(),
            }
        }

        [
            [
                // front plane
                Plane {
                    normal: Vector3::new(0.0, 0.0, -1.0),
                    d: -znear,
                },
                // back plane
                Plane {
                    normal: Vector3::new(0.0, 0.0, 1.0),
                    d: zfar,
                },
            ],
            [
                // right plane
                get_side_plane(near_right, -up),
                // left plane
                get_side_plane(near_left, up),
            ],
            [
                // top plane
                get_side_plane(near_top, right),
                // bottom plane
                get_side_plane(near_bottom, -right),
            ],
        ]
    }

    /// Checks whether the frustum contains the chunk. This function may return false positives.
    pub fn contains_chunk(
        planes: &[[Plane; 2]; 3],
        view_matrix: &Matrix4<f64>,
        chunk_pos: ChunkPos,
    ) -> bool {
        #[inline(always)]
        fn to_chunk_center(chunk_pos: i64) -> f64 {
            (chunk_pos * CHUNK_SIZE as i64 + CHUNK_SIZE as i64 / 2) as f64
        }
        #[inline(always)]
        fn to_vec3(v: Vector4<f64>) -> Vector3<f64> {
            Vector3::new(v.x / v.w, v.y / v.w, v.z / v.w)
        }
        let chunk_center = Vector4::new(
            to_chunk_center(chunk_pos.px),
            to_chunk_center(chunk_pos.py),
            to_chunk_center(chunk_pos.pz),
            1.0,
        );
        let chunk_center = to_vec3(view_matrix * chunk_center);
        let radius = CHUNK_SIZE as f64 * 3.0f64.sqrt() / 2.0;
        let mut keep = false;
        for [plane1, plane2] in planes.iter() {
            let d1 = plane1.dist(chunk_center);
            let d2 = plane2.dist(chunk_center);
            if d1 > 0.0 && d2 > 0.0 {
                // inside both
                keep = true;
            } else if d1.abs().max(d2.abs()) < radius {
                // close enough to the planes
                keep = true;
            }
        }
        keep
    }
}
