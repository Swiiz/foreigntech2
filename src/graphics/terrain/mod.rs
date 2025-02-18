/*
  Use octahedral mapping to map chunks to sphere?
  use sdf for terrain generation?
*/

use wgpu::{include_wgsl, BindGroup, DepthStencilState, RenderBundle, RenderBundleDepthStencil};

use super::{
    camera::{inv_view_proj_bind_group_layout, CameraUniform},
    ctx::GraphicsCtx,
    utils::TextureWrapper,
};

pub struct TerrainRenderer {
    pub(super) render_bundle: RenderBundle,
}

impl TerrainRenderer {
    pub fn new(ctx: &GraphicsCtx, camera: &CameraUniform) -> Self {
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&inv_view_proj_bind_group_layout(ctx)],
                push_constant_ranges: &[],
            });

        let shader = ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                    unclipped_depth: false,
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureWrapper::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.surface_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

        let mut encoder =
            ctx.device
                .create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                    label: None,
                    color_formats: &[Some(ctx.surface_format)],
                    depth_stencil: Some(RenderBundleDepthStencil {
                        depth_read_only: false,
                        stencil_read_only: false,
                        format: TextureWrapper::DEPTH_FORMAT,
                    }),
                    multiview: None,
                    sample_count: 1,
                });

        encoder.set_pipeline(&pipeline);
        encoder.set_bind_group(0, &camera.inv_view_proj_bindgroup, &[]);
        encoder.draw(0..6, 0..1);

        let render_bundle = encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("TerrainRenderer"),
        });

        Self { render_bundle }
    }
}
