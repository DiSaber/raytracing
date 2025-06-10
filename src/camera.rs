use crate::transform::Transform;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub fov: f32,
    pub near_clip: f32,
    pub far_clip: f32,
    pub transform: Transform,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: 90.0,
            near_clip: 0.1,
            far_clip: 1000.0,
            transform: Transform::default(),
        }
    }
}
