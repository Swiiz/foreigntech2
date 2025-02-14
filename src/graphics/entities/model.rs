use std::{
    io::{BufReader, Cursor},
    sync::atomic::{AtomicU32, Ordering},
    u16,
};

use nalgebra::Matrix4;
use tobj::Mesh;
use wgpu::util::DrawIndexedIndirectArgs;

use crate::{
    graphics::{
        buffer::{
            ColumnChange, CommonBuffer, DenseMapped2d, IndexBuffer, IndirectBuffer, InstanceBuffer,
            Slot2dId, StorageBuffer, VertexBuffer,
        },
        color::Color3,
        ctx::GraphicsCtx,
    },
    utils::DenseArrayOp,
    ASSETS,
};

use super::EntityModel;

pub struct ModelsAllocator {}

pub struct ModelsBuffer {
    pub(super) vertex_buffer: VertexBuffer<ModelVertex>,
    pub(super) index_buffer: IndexBuffer<u16>,
    pub(super) instance_buffer: DenseMapped2d<InstanceBuffer<ModelInstance>>,
    pub(super) indirect_buffer: IndirectBuffer,

    instances_count: Vec<Vec<u16>>,
    triangles_count: u32,
}

enum InstancesChange {
    Add {
        model_id: u16,
        mesh_id: u16,
        instance: ModelInstance,
        alloc_len: u32,
    },
    Remove {
        id: ModelInstanceId,
        array_op: DenseArrayOp,
    },
}

pub struct ModelInstanceId {
    pub model_id: u16,
    pub mesh_id: u16,
    pub instance_id: Slot2dId,
}

impl ModelsBuffer {
    pub fn from_raw(
        ctx: &GraphicsCtx,
        vertices: &[ModelVertex],
        indices: &[u16],
        instances: &[ModelInstance],
        indirects: &[wgpu::util::DrawIndexedIndirectArgs],
        instances_count: Vec<Vec<u16>>,
    ) -> Self {
        let vertex_buffer = VertexBuffer::new_const_array("Models vertices", ctx, vertices);
        let index_buffer = IndexBuffer::new_const_array("Models indices", ctx, indices);
        let instance_buffer = DenseMapped2d::new(
            "Models instances",
            ctx,
            instances,
            instances_count.iter().flatten().copied(),
        );
        let indirect_buffer =
            IndirectBuffer::new_array("Models index indirect args", ctx, indirects);

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            indirect_buffer,
            instances_count,
            triangles_count: indices.len() as u32 / 3 * instances.len() as u32,
        }
    }

    pub fn new<'a>(
        ctx: &GraphicsCtx,
        iter: impl IntoIterator<Item = (&'a Vec<Mesh>, Vec<Vec<ModelInstance>>)>,
    ) -> Self {
        let idx_counter = AtomicU32::new(0);
        let vtx_counter = AtomicU32::new(0);
        let inst_counter = AtomicU32::new(0);

        struct PerModel<T> {
            meshes: T,
        }

        struct PerMesh<T> {
            geometry: T,
            indirect: DrawIndexedIndirectArgs,
            instances: Vec<ModelInstance>,
        }

        let (vertices, indices, indirect, instances, instances_count) =
            iter.into_iter()
                .map(|(meshes, instances)| {
                    let meshes = meshes.into_iter().zip(instances).map(|(mesh, instances)| {
                        let vertices = (0..mesh.positions.len() / 3).map(|i| {
                            if mesh.normals.is_empty() {
                                ModelVertex {
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
                                ModelVertex {
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
                        });

                        let indices = mesh.indices.iter().map(|i| *i as u16);

                        let indirect = wgpu::util::DrawIndexedIndirectArgs {
                            index_count: mesh.indices.len() as u32,
                            instance_count: instances.len() as u32,
                            first_index: idx_counter
                                .fetch_add(mesh.indices.len() as u32, Ordering::SeqCst),
                            base_vertex: vtx_counter
                                .fetch_add(mesh.positions.len() as u32 / 3, Ordering::SeqCst)
                                as i32,
                            first_instance: inst_counter
                                .fetch_add(instances.len() as u32, Ordering::SeqCst),
                        };

                        PerMesh {
                            geometry: (vertices, indices),
                            indirect,
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
                        mut instances_count,
                    ): (Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>),
                     model| {
                        let mut instance_count = Vec::with_capacity(model.meshes.len());
                        for mesh in model.meshes {
                            let (local_vertices, local_indices) = mesh.geometry;
                            vertices.extend(local_vertices);
                            indices.extend(local_indices);
                            instance_count.push(mesh.instances.len() as u16);
                            instances.extend(mesh.instances);
                            indirect.push(mesh.indirect);
                        }

                        instances_count.push(instance_count);

                        (vertices, indices, indirect, instances, instances_count)
                    },
                );

        Self::from_raw(
            ctx,
            &vertices,
            &indices,
            &instances,
            &indirect,
            instances_count,
        )
    }

    pub fn triangles_count(&self) -> u32 {
        self.triangles_count
    }

    pub fn add_instance(
        &mut self,
        model_id: u16,
        mesh_id: u16,
        instance: ModelInstance,
    ) -> ModelInstanceId {
        let column_id = self.instances_count[..model_id as usize]
            .iter()
            .flatten()
            .sum::<u16>()
            + mesh_id as u16;
        let instance_id = self.instance_buffer.push(column_id, instance);
        self.instances_count[model_id as usize][mesh_id as usize] += 1;

        ModelInstanceId {
            model_id,
            mesh_id,
            instance_id,
        }
    }

    pub fn remove_instance(&mut self, id: ModelInstanceId) {
        self.instance_buffer.remove(id.instance_id);
        self.instances_count[id.model_id as usize][id.mesh_id as usize] -= 1;
    }

    pub fn model_count(&self) -> u32 {
        self.instances_count[..]
            .iter()
            .map(|m| m.len())
            .sum::<usize>() as u32
    }

    //TODO: Use staging belt please
    pub fn apply_changes(&mut self, ctx: &GraphicsCtx) {
        let (_grown, changes) = self.instance_buffer.apply_changes(ctx);

        for (column_id, change) in changes {
            match change {
                ColumnChange::Moved { new_offset } => {
                    self.indirect_buffer.write_first_instance_at_index(
                        ctx,
                        column_id as u32,
                        new_offset as u32,
                    );
                }
                ColumnChange::Resized { new_size } => {
                    self.indirect_buffer.write_instance_count_at_index(
                        ctx,
                        column_id as u32,
                        new_size as u32,
                    );
                }
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl ModelVertex {
    pub fn buffer_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
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
                visibility: wgpu::ShaderStages::FRAGMENT,
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
    pub diffuse_color: [f32; 3],

    pub diffuse_texture_id: u32,
}

pub fn load_model(model_name: &str) -> EntityModel {
    let model_file = ASSETS.models.get(model_name).unwrap();
    let obj_cursor = Cursor::new(model_file.0.clone());
    let mut obj_reader = BufReader::new(obj_cursor);
    let (models, mat_res) = tobj::load_obj_buf(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| {
            let material = p
                .to_str()
                .unwrap_or_else(|| panic!("Invalid material name {p:?} in model {model_name}"))
                .strip_suffix(".mtl")
                .expect("Invalid material file type {m:?} in model {model_name}. Expected .mtl");
            let material_file = ASSETS
                .materials
                .get(&material)
                .unwrap_or_else(|| panic!("Failed to load material {material}"));
            let obj_cursor = Cursor::new(material_file.0.clone());
            let mut obj_reader = BufReader::new(obj_cursor);
            tobj::load_mtl_buf(&mut obj_reader)
        },
    )
    .expect("Failed to load model");
    let materials: Vec<_> = mat_res.expect("Failed to load materials");

    EntityModel {
        meshes: models.into_iter().map(|m| m.mesh).collect(),
        textures: materials
            .iter()
            .filter_map(|m| {
                let texture_file = m
                    .diffuse_texture
                    .as_ref()?;
                let texture = texture_file
                    .strip_suffix(".png")
                    .or(texture_file.strip_prefix(".jpg"))
                    .expect("Invalid texture file type {m:?} in model {model_name}. Expected .png or .jpg");
                Some(ASSETS.textures.get(texture).unwrap().0.clone())
            })
            .collect(),
        materials: materials
            .into_iter()
            .map(|m| Material {
                diffuse_color: m.diffuse.unwrap_or(Color3::WHITE.into()),
                diffuse_texture_id: match  m.diffuse_texture {
                    None => u32::MAX,
                    Some(_) => 0,
                },
            })
            .collect(),
    }
}
