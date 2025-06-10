use crate::{dense_storage::DenseStorageIndex, transform::Transform};

#[derive(Debug, Clone, Copy)]
pub struct MeshObject {
    pub mesh: DenseStorageIndex,
    pub material: DenseStorageIndex,
    pub transform: Transform,
}
