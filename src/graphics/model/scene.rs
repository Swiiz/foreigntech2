use std::f32::consts::E;

use nalgebra::Matrix4;
use tobj::{Mesh, Model};

use crate::graphics::{color::Color3, ctx::GraphicsCtx, utils::TextureWrapper};

use super::EntityModel;

pub struct ModelsBuffer {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) instance_buffer: wgpu::Buffer,
    pub(super) indirect_buffer: wgpu::Buffer,

    triangles_count: u32,
}

impl ModelsBuffer {
    pub fn from_raw(
        ctx: &GraphicsCtx,
        vertices: &[Vertex],
        indices: &[u16],
        instances: &[ModelInstance],
        indirect: &[wgpu::util::DrawIndexedIndirectArgs],
    ) -> Self {
        let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Models Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Models Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            },
        );

        let instance_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Models Instance Buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let indirect_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Models Indirect Buffer"),
                // SAFETY: `DrawIndexedIndirectArgs` is repr(C) and made to be casted to `[u32; _]`
                contents: unsafe {
                    std::slice::from_raw_parts(
                        indirect.as_ptr().cast(),
                        indirect.len() * std::mem::size_of::<wgpu::util::DrawIndexedIndirectArgs>(),
                    )
                },
                usage: wgpu::BufferUsages::INDIRECT,
            },
        );

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            indirect_buffer,
            triangles_count: indices.len() as u32 / 3 * instances.len() as u32,
        }
    }

    //TODO: make instancing dynamic
    pub fn new(ctx: &GraphicsCtx, models: &[EntityModel], instances: &[&[ModelInstance]]) -> Self {
        let mut idx_counter = 0;
        let mut inst_counter = 0;

        let (vertices, indices, indirect) = models
            .into_iter()
            .zip(instances)
            .map(|(EntityModel { meshes }, instances)| {
                let res = meshes.into_iter().map(move |mesh| {
                    (
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
                        wgpu::util::DrawIndexedIndirectArgs {
                            index_count: mesh.indices.len() as u32,
                            instance_count: instances.len() as u32,
                            first_index: 0,
                            base_vertex: {
                                let i = idx_counter as i32;
                                idx_counter += mesh.indices.len() as u32;
                                i
                            },
                            first_instance: inst_counter,
                        },
                    )
                });

                inst_counter += instances.len() as u32;

                res
            })
            .flatten()
            .fold(
                Default::default(),
                |(mut a, mut b, mut c): (Vec<_>, Vec<_>, Vec<_>), (x, y, z)| {
                    a.extend(x);
                    b.extend(y);
                    c.push(z);
                    (a, b, c)
                },
            );

        let instances: Vec<_> = instances.iter().map(|i| *i).flatten().map(|i| *i).collect();

        Self::from_raw(ctx, &vertices, &indices, &instances, &indirect)
    }

    pub fn triangles_count(&self) -> u32 {
        self.triangles_count
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

pub struct MaterialsUniform {
    pub storage_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl MaterialsUniform {
    pub fn new(ctx: &GraphicsCtx, materials: &[Material]) -> Self {
        let storage_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Materials Storage Buffer"),
                contents: bytemuck::cast_slice(materials),
                usage: wgpu::BufferUsages::STORAGE,
            },
        );

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &materials_buffer_bind_group_layout(ctx),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer.as_entire_binding(),
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
