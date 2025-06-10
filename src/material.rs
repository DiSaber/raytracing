use glam::Vec3;

#[derive(Debug, Default, Clone, Copy)]
pub struct Material {
    pub albedo: Vec3,
    pub emissive: Vec3,
    pub emissive_strength: f32,
}
