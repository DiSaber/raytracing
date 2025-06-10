use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

use crate::{material::Material, mesh::Vertex};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    pub view_inverse: Mat4,
    pub proj_inverse: Mat4,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct GpuVertex {
    pub pos: Vec3,
    _p0: u32,
    pub normal: Vec3,
    _p1: u32,
}

impl From<Vertex> for GpuVertex {
    fn from(value: Vertex) -> Self {
        Self {
            pos: value.pos,
            normal: value.normal,
            ..Default::default()
        }
    }
}

impl From<&Vertex> for GpuVertex {
    fn from(value: &Vertex) -> Self {
        Self {
            pos: value.pos,
            normal: value.normal,
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuInstance {
    pub first_vertex: u32,
    pub first_index: u32,
    pub material_index: u32,
    pub _p0: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug)]
pub struct GpuMaterial {
    pub albedo: Vec3,
    pub _p1: u32,
    pub emissive: Vec3,
    pub emissive_strength: f32,
}

impl From<Material> for GpuMaterial {
    fn from(value: Material) -> Self {
        Self {
            albedo: value.albedo,
            emissive: value.emissive,
            emissive_strength: value.emissive_strength,
            ..Default::default()
        }
    }
}

impl From<&Material> for GpuMaterial {
    fn from(value: &Material) -> Self {
        Self {
            albedo: value.albedo,
            emissive: value.emissive,
            emissive_strength: value.emissive_strength,
            ..Default::default()
        }
    }
}
