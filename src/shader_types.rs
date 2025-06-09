use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct Vertex {
    pub pos: Vec3,
    _p0: u32,
    pub normal: Vec3,
    _p1: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    pub view_inverse: Mat4,
    pub proj_inverse: Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct InstanceEntry {
    pub first_vertex: u32,
    pub first_geometry: u32,
    pub last_geometry: u32,
    pub _p0: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct GeometryEntry {
    pub first_index: u32,
    pub _p0: [u32; 3],
    pub material: Material,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug)]
pub struct Material {
    pub roughness_exponent: f32,
    pub metalness: f32,
    pub specularity: f32,
    pub _p0: u32,
    pub albedo: Vec3,
    pub _p1: u32,
    pub emissive: Vec3,
    pub emissive_strength: f32,
}

#[derive(Debug, Clone, Default)]
pub struct RawSceneComponents {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub geometries: Vec<(Range<usize>, Material)>, // index range, material
    pub instances: Vec<(Range<usize>, Range<usize>)>, // vertex range, geometry range
}

impl RawSceneComponents {
    /// Inserts an obj with a material. Returns `None` if an error occurred.
    pub fn insert_obj(&mut self, path: &str, material: Material) -> Option<()> {
        let start_vertex = self.vertices.len();
        let start_geometry = self.geometries.len();

        let (models, _) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS).ok()?;

        for model in models {
            let mut mesh = model.mesh;

            self.vertices.extend(
                mesh.positions
                    .chunks_exact(3)
                    .zip(mesh.normals.chunks_exact(3))
                    .map(|(pos, normal)| Vertex {
                        pos: Vec3::from_slice(pos),
                        normal: Vec3::from_slice(normal),
                        ..Default::default()
                    }),
            );
            let start_index = self.indices.len();
            self.indices.append(&mut mesh.indices);
            self.geometries
                .push((start_index..self.indices.len(), material));
        }

        self.instances.push((
            start_vertex..self.vertices.len(),
            start_geometry..self.geometries.len(),
        ));

        Some(())
    }

    /// Uploads the scene to the gpu. Ensure `wgpu::Features::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE` is enabled.
    pub fn upload_scene(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> SceneComponents {
        let geometry_buffer_content = self
            .geometries
            .iter()
            .map(|(index_range, material)| GeometryEntry {
                first_index: index_range.start as u32,
                material: *material,
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let instance_buffer_content = self
            .instances
            .iter()
            .map(|geometry| InstanceEntry {
                first_vertex: geometry.0.start as u32,
                first_geometry: geometry.1.start as u32,
                last_geometry: geometry.1.end as u32,
                _p0: 1,
            })
            .collect::<Vec<_>>();

        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertices"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::BLAS_INPUT,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Indices"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::BLAS_INPUT,
        });
        let geometries = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Geometries"),
            contents: bytemuck::cast_slice(&geometry_buffer_content),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instances"),
            contents: bytemuck::cast_slice(&instance_buffer_content),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let (size_descriptors, bottom_level_acceleration_structures): (Vec<_>, Vec<_>) = self
            .instances
            .iter()
            .map(|(vertex_range, geometry_range)| {
                let size_desc: Vec<wgpu::BlasTriangleGeometrySizeDescriptor> = (*geometry_range)
                    .clone()
                    .map(|i| wgpu::BlasTriangleGeometrySizeDescriptor {
                        vertex_format: wgpu::VertexFormat::Float32x3,
                        vertex_count: vertex_range.end as u32 - vertex_range.start as u32,
                        index_format: Some(wgpu::IndexFormat::Uint32),
                        index_count: Some(
                            self.geometries[i].0.end as u32 - self.geometries[i].0.start as u32,
                        ),
                        flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
                    })
                    .collect();

                let blas = device.create_blas(
                    &wgpu::CreateBlasDescriptor {
                        label: None,
                        flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                        update_mode: wgpu::AccelerationStructureUpdateMode::Build,
                    },
                    wgpu::BlasGeometrySizeDescriptors::Triangles {
                        descriptors: size_desc.clone(),
                    },
                );
                (size_desc, blas)
            })
            .unzip();

        let build_entries: Vec<_> = self
            .instances
            .iter()
            .zip(size_descriptors.iter())
            .zip(bottom_level_acceleration_structures.iter())
            .map(|(((vertex_range, geometry_range), size_desc), blas)| {
                let triangle_geometries: Vec<_> = size_desc
                    .iter()
                    .zip(geometry_range.clone())
                    .map(|(size, i)| wgpu::BlasTriangleGeometry {
                        size,
                        vertex_buffer: &vertices,
                        first_vertex: vertex_range.start as u32,
                        vertex_stride: std::mem::size_of::<Vertex>() as u64,
                        index_buffer: Some(&indices),
                        first_index: Some(self.geometries[i].0.start as u32),
                        transform_buffer: None,
                        transform_buffer_offset: None,
                    })
                    .collect();

                wgpu::BlasBuildEntry {
                    blas,
                    geometry: wgpu::BlasGeometries::TriangleGeometries(triangle_geometries),
                }
            })
            .collect();

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        encoder.build_acceleration_structures(build_entries.iter(), std::iter::empty());

        queue.submit(Some(encoder.finish()));

        SceneComponents {
            vertices,
            indices,
            geometries,
            instances,
            bottom_level_acceleration_structures,
        }
    }
}

pub struct SceneComponents {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub geometries: wgpu::Buffer,
    pub instances: wgpu::Buffer,
    pub bottom_level_acceleration_structures: Vec<wgpu::Blas>,
}
