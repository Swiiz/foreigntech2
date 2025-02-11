use nalgebra::Matrix4;
use wgpu::util::DrawIndexedIndirectArgs;

use crate::{
    graphics::{
        buffer::{
            CommonBuffer, Growable, IndexBuffer, IndirectBuffer, InstanceBuffer, Mapped,
            StorageBuffer, VertexBuffer,
        },
        color::Color3,
        ctx::GraphicsCtx,
    },
    utils::IdAllocator,
};

use super::EntityModel;

pub struct ModelsAllocator {}

pub struct ModelsBuffer {
    pub(super) vertex_buffer: VertexBuffer<Vertex>,
    pub(super) index_buffer: IndexBuffer<u16>,
    pub(super) instance_buffer: Growable<InstanceBuffer<ModelInstance>>,
    pub(super) indirect_buffer: Growable<IndirectBuffer>,

    instances_ids: Vec<Vec<IdAllocator<u16>>>,
    meshes_count: Vec<u16>,

    triangles_count: u32,
}

pub struct ModelInstanceId {
    pub model_id: u16,
    pub mesh_id: u16,
    pub instance_id: u16,
}

impl ModelsBuffer {
    pub fn from_raw(
        ctx: &GraphicsCtx,
        vertices: &[Vertex],
        indices: &[u16],
        instances: &[ModelInstance],
        instances_ids: Vec<Vec<IdAllocator<u16>>>,
        meshes_count: Vec<u16>,
        indirects: &[wgpu::util::DrawIndexedIndirectArgs],
    ) -> Self {
        let vertex_buffer = VertexBuffer::new_const_array("Models vertices", ctx, vertices);
        let index_buffer = IndexBuffer::new_const_array("Models indices", ctx, indices);
        let instance_buffer = InstanceBuffer::new_vec("Models instances", ctx, instances);
        let indirect_buffer = IndirectBuffer::new_vec("Models call params", ctx, indirects);

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            instances_ids,
            meshes_count,
            indirect_buffer,
            triangles_count: indices.len() as u32 / 3 * instances.len() as u32,
        }
    }

    pub fn new<'a>(
        ctx: &GraphicsCtx,
        iter: impl IntoIterator<Item = (&'a EntityModel, Vec<Vec<ModelInstance>>)>,
    ) -> Self {
        let mut idx_counter = 0;
        let mut inst_counter = 0;

        struct PerModel<T> {
            meshes: T,
        }

        struct PerMesh<T> {
            geometry: T,
            indirect: DrawIndexedIndirectArgs,
            instances: Vec<ModelInstance>,
            instances_ids: IdAllocator<u16>,
        }

        //TODO: per mesh instancing instead of per model
        let (vertices, indices, indirect, instances, instances_ids, meshes_count) = iter
            .into_iter()
            .map(|(EntityModel { meshes }, instances)| {
                let mesh_count = meshes.len() as u16;
                let meshes = meshes
                    .into_iter()
                    .zip(instances)
                    .map(move |(mesh, instances)| {
                        let geometry = (
                            (0..mesh.positions.len() / 3).map(|i| {
                                if mesh.normals.is_empty() {
                                    Vertex {
                                        position: [
                                            mesh.positions[i * 3],
                                            mesh.positions[i * 3 + 1],
                                            mesh.positions[i * 3 + 2],
                                        ],
                                        tex_coords: [
                                            mesh.texcoords[i * 2],
                                            1.0 - mesh.texcoords[i * 2 + 1],
                                        ],
                                        normal: [0.0, 0.0, 0.0],
                                    }
                                } else {
                                    Vertex {
                                        position: [
                                            mesh.positions[i * 3],
                                            mesh.positions[i * 3 + 1],
                                            mesh.positions[i * 3 + 2],
                                        ],
                                        tex_coords: [
                                            mesh.texcoords[i * 2],
                                            1.0 - mesh.texcoords[i * 2 + 1],
                                        ],
                                        normal: [
                                            mesh.normals[i * 3],
                                            mesh.normals[i * 3 + 1],
                                            mesh.normals[i * 3 + 2],
                                        ],
                                    }
                                }
                            }),
                            mesh.indices.iter().map(|i| *i as u16),
                        );

                        let indirect = wgpu::util::DrawIndexedIndirectArgs {
                            index_count: mesh.indices.len() as u32,
                            instance_count: instances.len() as u32,
                            first_index: 0,
                            base_vertex: idx_counter as i32,
                            first_instance: inst_counter,
                        };

                        idx_counter += mesh.indices.len() as u32;
                        inst_counter += instances.len() as u32;

                        PerMesh {
                            geometry,
                            indirect,
                            //TODO: change for new instancing
                            instances_ids: IdAllocator::new_packed(instances.len() as u16),
                            instances,
                        }
                    });

                PerModel { meshes }
            })
            .fold(
                Default::default(),
                |(
                    mut vertices,
                    mut indices,
                    mut indirect,
                    mut instances,
                    mut instances_ids,
                    mut meshes_count,
                ): (Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>),
                 model| {
                    meshes_count.push(model.meshes.len() as u16);

                    let mut meshes_instance_ids = Vec::with_capacity(model.meshes.len());
                    for mesh in model.meshes {
                        vertices.extend(mesh.geometry.0);
                        indices.extend(mesh.geometry.1);
                        instances.extend(mesh.instances);
                        indirect.push(mesh.indirect);
                        meshes_instance_ids.push(mesh.instances_ids);
                    }
                    instances_ids.push(meshes_instance_ids);

                    (
                        vertices,
                        indices,
                        indirect,
                        instances,
                        instances_ids,
                        meshes_count,
                    )
                },
            );

        Self::from_raw(
            ctx,
            &vertices,
            &indices,
            &instances,
            instances_ids,
            meshes_count,
            &indirect,
        )
    }

    pub fn triangles_count(&self) -> u32 {
        self.triangles_count
    }

    pub fn add_instance(
        &mut self,
        model_id: u16,
        mesh_id: u16,
        instance: &ModelInstance,
    ) -> ModelInstanceId {
        let instance_id = self.instances_ids[model_id as usize][mesh_id as usize].allocate();
        // FRAGMENTATION OF IDS IS NOT ALLOWED!
        // Array of SlotAllocator
        //TODO: insert into buffer
        ModelInstanceId {
            model_id,
            mesh_id,
            instance_id,
        }
    }

    pub fn remove_instance(&mut self, id: ModelInstanceId) {
        self.instances_ids[id.model_id as usize][id.mesh_id as usize].free(id.instance_id);
        //TODO: remove from buffer
    }

    pub fn apply_changes(&mut self, ctx: &GraphicsCtx) -> bool {
        //Todo: self.instance_buffer.maybe_grow(ctx, required_size) || self.indirect_buffer.maybe_grow(ctx, required_size)
        false
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex {
    pub fn buffer_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelInstance {
    pub transform: [[f32; 4]; 4],
    pub material_id: u32,
}

impl ModelInstance {
    pub fn new(transform: Matrix4<f32>, material_id: u32) -> Self {
        Self {
            transform: transform.into(),
            material_id,
        }
    }

    pub fn buffer_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

pub struct MaterialsBuffer {
    pub storage_buffer: StorageBuffer<Material>,
    pub bind_group: wgpu::BindGroup,
}

impl MaterialsBuffer {
    pub fn new(ctx: &GraphicsCtx, materials: &[Material]) -> Self {
        let storage_buffer = StorageBuffer::new_array("Materials", ctx, materials);

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &materials_buffer_bind_group_layout(ctx),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer.binding(),
            }],
            label: Some("Materials Bind Group"),
        });

        Self {
            storage_buffer,
            bind_group,
        }
    }
}

pub fn materials_buffer_bind_group_layout(ctx: &GraphicsCtx) -> wgpu::BindGroupLayout {
    ctx.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Materials Bind Group Layout"),
        })
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Material {
    pub color: [f32; 4],
}

impl Material {
    pub fn new(color: Color3) -> Self {
        Self {
            color: color.into(),
        }
    }
}
