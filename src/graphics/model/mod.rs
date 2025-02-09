use std::rc::Rc;

use tobj::Mesh;

pub mod renderer;
pub mod scene;

pub struct EntityModel {
    pub meshes: Vec<Mesh>,
    //pub materials: Vec<Material>,
}
