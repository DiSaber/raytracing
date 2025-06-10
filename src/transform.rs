use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub translation: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            scale: Vec3::ONE,
            rotation: Quat::IDENTITY,
        }
    }
}

impl From<Transform> for Mat4 {
    fn from(value: Transform) -> Self {
        Self::from_scale_rotation_translation(value.scale, value.rotation, value.translation)
    }
}

impl From<&Transform> for Mat4 {
    fn from(value: &Transform) -> Self {
        Self::from_scale_rotation_translation(value.scale, value.rotation, value.translation)
    }
}
