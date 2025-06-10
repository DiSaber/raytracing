use glam::Vec3;
use wgpu::naga::FastHashMap;

use crate::{
    dense_storage::{DenseStorage, DenseStorageIndex},
    material::Material,
    mesh::{Mesh, Vertex},
    mesh_object::MeshObject,
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
        let mut instances = FastHashMap::<_, Vec<Transform>>::default();

        for mesh_object in self
            .mesh_objects
            .into_iter()
            .filter_map(|(_, mesh_object)| mesh_object)
        {
            instances
                .entry((mesh_object.mesh, mesh_object.material))
                .or_default()
                .push(mesh_object.transform);
        }

        // println!("{:?}", instances);

        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct GpuScene {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub materials: wgpu::Buffer,
    pub instances: wgpu::Buffer,
    pub bottom_level_acceleration_structures: Vec<wgpu::Blas>,
}
