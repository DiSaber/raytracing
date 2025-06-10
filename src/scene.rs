use glam::Vec3;
use wgpu::{naga::FastHashMap, util::DeviceExt};

use crate::{
    dense_storage::{DenseStorage, DenseStorageIndex},
    material::Material,
    mesh::{Mesh, Vertex},
    mesh_object::MeshObject,
    shader_types::{GpuInstance, GpuMaterial, GpuVertex},
    transform::Transform,
};

/// A scene that contains mesh objects and their meshes/materials
#[derive(Debug, Default, Clone)]
pub struct Scene {
    meshes: DenseStorage<Mesh>,
    materials: DenseStorage<Material>,
    mesh_objects: DenseStorage<MeshObject>,
}

impl Scene {
    /// Loads a mesh and returns a handle if successful
    pub fn load_mesh(&mut self, path: &str) -> Option<DenseStorageIndex> {
        let (models, _) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS).ok()?;

        let model = models.first()?;

        Some(
            self.meshes.push(Mesh {
                vertices: model
                    .mesh
                    .positions
                    .chunks_exact(3)
                    .zip(model.mesh.normals.chunks_exact(3))
                    .map(|(pos, normal)| Vertex {
                        pos: Vec3::from_slice(pos),
                        normal: Vec3::from_slice(normal),
                    })
                    .collect(),
                indices: model.mesh.indices.clone(),
            }),
        )
    }

    /// Inserts a material and returns a handle
    pub fn insert_material(&mut self, material: Material) -> DenseStorageIndex {
        self.materials.push(material)
    }

    /// Inserts a mesh object and returns a handle
    pub fn insert_mesh_object(&mut self, mesh_object: MeshObject) -> DenseStorageIndex {
        self.mesh_objects.push(mesh_object)
    }

    /// Uploads the scene to the gpu
    pub fn upload_to_gpu(self, device: &wgpu::Device, queue: &wgpu::Queue) -> GpuScene {
        let mut mesh_objects = FastHashMap::<_, Vec<Transform>>::default();

        for mesh_object in self
            .mesh_objects
            .into_iter()
            .filter_map(|(_, mesh_object)| mesh_object)
        {
            mesh_objects
                .entry((mesh_object.mesh, mesh_object.material))
                .or_default()
                .push(mesh_object.transform);
        }

        let mesh_objects = mesh_objects
            .into_iter()
            .filter_map(|((mesh, material), transforms)| {
                match (self.meshes.get(mesh), self.materials.get(material)) {
                    (Some(mesh), Some(material)) => Some((mesh, material, transforms)),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut instances = Vec::with_capacity(mesh_objects.len());
        let mut materials = Vec::with_capacity(mesh_objects.len());
        let mut instance_transforms = Vec::with_capacity(mesh_objects.len());

        for (mesh, material, transforms) in mesh_objects {
            let start_vertex = vertices.len();
            let start_index = indices.len();

            vertices.extend(mesh.vertices.iter().map(GpuVertex::from));
            indices.extend_from_slice(&mesh.indices);

            instances.push((start_vertex..vertices.len(), start_index..indices.len()));
            materials.push(GpuMaterial::from(material));
            instance_transforms.push(transforms);
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::BLAS_INPUT,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::BLAS_INPUT,
        });
        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Materials"),
            contents: bytemuck::cast_slice(&materials),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instances"),
            contents: bytemuck::cast_slice(
                &instances
                    .iter()
                    .enumerate()
                    .map(|(i, (vertex_range, index_range))| GpuInstance {
                        first_vertex: vertex_range.start as u32,
                        first_index: index_range.start as u32,
                        material_index: i as u32,
                        _p0: 0,
                    })
                    .collect::<Vec<_>>(),
            ),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let (size_descriptors, bottom_level_acceleration_structures): (Vec<_>, Vec<_>) = instances
            .iter()
            .map(|(vertex_range, index_range)| {
                let size_desc = wgpu::BlasTriangleGeometrySizeDescriptor {
                    vertex_format: wgpu::VertexFormat::Float32x3,
                    vertex_count: vertex_range.end as u32 - vertex_range.start as u32,
                    index_format: Some(wgpu::IndexFormat::Uint32),
                    index_count: Some(index_range.end as u32 - index_range.start as u32),
                    flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
                };

                let blas = device.create_blas(
                    &wgpu::CreateBlasDescriptor {
                        label: None,
                        flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                        update_mode: wgpu::AccelerationStructureUpdateMode::Build,
                    },
                    wgpu::BlasGeometrySizeDescriptors::Triangles {
                        descriptors: vec![size_desc.clone()],
                    },
                );

                (size_desc, blas)
            })
            .unzip();

        let build_entries: Vec<_> = instances
            .iter()
            .zip(size_descriptors.iter())
            .zip(bottom_level_acceleration_structures.iter())
            .map(|(((vertex_range, index_range), size_desc), blas)| {
                let triangle_geometries = wgpu::BlasTriangleGeometry {
                    size: size_desc,
                    vertex_buffer: &vertex_buffer,
                    first_vertex: vertex_range.start as u32,
                    vertex_stride: std::mem::size_of::<GpuVertex>() as u64,
                    index_buffer: Some(&index_buffer),
                    first_index: Some(index_range.start as u32),
                    transform_buffer: None,
                    transform_buffer_offset: None,
                };

                wgpu::BlasBuildEntry {
                    blas,
                    geometry: wgpu::BlasGeometries::TriangleGeometries(vec![triangle_geometries]),
                }
            })
            .collect();

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        encoder.build_acceleration_structures(build_entries.iter(), std::iter::empty());

        queue.submit(Some(encoder.finish()));

        GpuScene {
            vertex_buffer,
            index_buffer,
            material_buffer,
            instance_buffer,
            instance_transforms,
            bottom_level_acceleration_structures,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuScene {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub material_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instance_transforms: Vec<Vec<Transform>>,
    pub bottom_level_acceleration_structures: Vec<wgpu::Blas>,
}
