use std::collections::HashMap;

use guillotiere::{size2, AllocId, AtlasAllocator};
use image::{imageops::overlay, EncodableLayout, RgbImage, RgbaImage};
use wgpu::util::DeviceExt;

use crate::graphics::{ctx::GraphicsCtx, utils::TextureWrapper};

pub struct AtlasPacker {
    atlas: AtlasAllocator,
    images: HashMap<AllocId, RgbaImage>,
    dims: (u32, u32),
}

impl AtlasPacker {
    pub fn new() -> Self {
        let dims = (2048, 2048);
        Self {
            //TODO: add auto growing of atlas
            atlas: AtlasAllocator::new(dims.into()),
            images: HashMap::new(),
            dims: (dims.0 as u32, dims.1 as u32),
        }
    }

    pub fn add_image(&mut self, image: impl Into<RgbaImage>) -> () {
        let image = image.into();
        let id = self
            .atlas
            .allocate(size2(image.width() as i32, image.height() as i32))
            .unwrap_or_else(|| panic!("Failed to allocate texture to atlas"))
            .id;
        self.images.insert(id, image);
    }

    pub fn build_atlas(&mut self, ctx: &GraphicsCtx) -> AtlasUniform {
        let (width, height) = self.dims;
        let mut texture = RgbaImage::new(width, height);
        let mut uvs = Vec::with_capacity(self.images.len());
        self.atlas.for_each_allocated_rectangle(|id, rectangle| {
            let image = self.images.get(&id).unwrap();
            overlay(
                &mut texture,
                image,
                rectangle.min.x as i64,
                rectangle.min.y as i64,
            );
            uvs.push([
                [
                    rectangle.min.x as f32 / width as f32,
                    rectangle.min.y as f32 / height as f32,
                ],
                [
                    rectangle.max.x as f32 / width as f32,
                    rectangle.max.y as f32 / height as f32,
                ],
            ]);
        });

        let texture =
            TextureWrapper::new_rgba_2d("Models Atlas", ctx, self.dims, texture.as_bytes());

        let uvs_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Atlas UVs"),
                contents: bytemuck::cast_slice(&uvs),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &atlas_uniform_bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uvs_buffer.as_entire_binding(),
                },
            ],
            label: Some("Atlas Bind Group"),
        });

        AtlasUniform {
            packer: self.atlas.clone(),
            texture,
            uvs_buffer,
            bind_group,
        }
    }
}

pub struct AtlasUniform {
    packer: AtlasAllocator,
    texture: TextureWrapper,
    uvs_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

pub fn atlas_uniform_bind_group_layout(ctx: &GraphicsCtx) -> wgpu::BindGroupLayout {
    ctx.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Atlas Bind Group Layout"),
        })
}
