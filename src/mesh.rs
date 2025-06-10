use glam::Vec3;

#[derive(Debug, Default, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Vertex {
    pub pos: Vec3,
    pub normal: Vec3,
}
