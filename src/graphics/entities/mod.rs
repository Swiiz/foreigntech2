use tobj::Mesh;

pub mod model;
pub mod renderer;

pub struct EntityModel {
    pub meshes: Vec<Mesh>,
    //pub materials: Vec<Material>,
}
