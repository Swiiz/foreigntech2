use std::collections::HashMap;

use guillotiere::{size2, AllocId, AtlasAllocator};
use image::{imageops::overlay, EncodableLayout, RgbImage};

use crate::graphics::{ctx::GraphicsCtx, utils::TextureWrapper};

pub struct AtlasPacker {
    atlas: AtlasAllocator,
    images: HashMap<AllocId, RgbImage>,
    dims: (u32, u32),
}

impl AtlasPacker {
    pub fn new() -> Self {
        let dims = (1024, 1024);
        Self {
            //TODO: add auto growing of atlas
            atlas: AtlasAllocator::new(dims.into()),
            images: HashMap::new(),
            dims: (dims.0 as u32, dims.1 as u32),
        }
    }

    pub fn add_image(&mut self, image: impl Into<RgbImage>) -> () {
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
        let mut texture = RgbImage::new(width, height);
        self.atlas.for_each_allocated_rectangle(|id, rectangle| {
            let image = self.images.get(&id).unwrap();
            overlay(
                &mut texture,
                image,
                rectangle.min.x as i64,
                rectangle.min.y as i64,
            );
        });

        let texture = TextureWrapper::new_2d("Models Atlas", ctx, self.dims, 3, texture.as_bytes());
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
            ],
            label: Some("Models Atlas Bind Group"),
        });

        AtlasUniform {
            packer: self.atlas.clone(),
            texture,
            bind_group,
        }
    }
}

pub struct AtlasUniform {
    packer: AtlasAllocator,
    texture: TextureWrapper,
    bind_group: wgpu::BindGroup,
}

fn atlas_uniform_bind_group_layout(ctx: &GraphicsCtx) -> wgpu::BindGroupLayout {
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
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        })
}
