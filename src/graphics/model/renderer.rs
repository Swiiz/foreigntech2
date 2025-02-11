use std::{
    cell::LazyCell,
    io::{BufReader, Cursor},
};

use nalgebra::{Matrix4, Point3, Vector3};
use nd_iter::iter_3d;
use wgpu::{
    core::pipeline, include_wgsl, DepthStencilState, RenderBundle, RenderBundleDepthStencil,
};

use crate::graphics::{
    atlas::{atlas_uniform_bind_group_layout, AtlasPacker, AtlasUniform},
    buffer::CommonBuffer,
    camera::view_proj_bind_group_layout,
    color::Color3,
    ctx::GraphicsCtx,
    light::{lights_buffer_bind_group_layout, Light, LightsBuffer},
    model::scene::{materials_buffer_bind_group_layout, Material},
    utils::TextureWrapper,
};

use super::{
    scene::{MaterialsBuffer, ModelInstance, ModelsBuffer, Vertex},
    EntityModel,
};

pub struct ModelRenderer {
    pub models: ModelsBuffer,
    pub materials: MaterialsBuffer,
    pub atlas: AtlasUniform,

    pipeline: wgpu::RenderPipeline,
    pub render_bundle: RenderBundle,
}

impl ModelRenderer {
    pub fn new(
        ctx: &GraphicsCtx,
        view_proj_bindgroup: &wgpu::BindGroup,
        lights: &LightsBuffer,
    ) -> Self {
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &view_proj_bind_group_layout(ctx),
                    &materials_buffer_bind_group_layout(ctx),
                    &atlas_uniform_bind_group_layout(ctx),
                    &lights_buffer_bind_group_layout(ctx),
                ],
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
                    buffers: &[Vertex::buffer_desc(), ModelInstance::buffer_desc()],
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
                    depth_compare: wgpu::CompareFunction::Less,
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

        let models = ModelsBuffer::new(
            ctx,
            [(&load_test_model(), vec![STRESS_TEST_INSTANCES.to_vec()])],
        );
        println!(
            "Models buffer configured to render {} triangles",
            models.triangles_count()
        );

        let atlas = {
            let mut packer = AtlasPacker::new();
            let image =
                image::load_from_memory(include_bytes!("../../../assets/Astronaut_BaseColor.png"))
                    .expect("Failed to load image")
                    .to_rgba8();
            packer.add_image(image);
            packer.build_atlas(ctx)
        };
        let materials = MaterialsBuffer::new(ctx, &[Material::new(Color3::WHITE)]);

        let render_bundle = create_models_render_bundle(
            &pipeline,
            &models,
            &materials,
            &atlas,
            ctx,
            view_proj_bindgroup,
            lights,
        );

        Self {
            models,
            materials,
            atlas,
            pipeline,
            render_bundle,
        }
    }

    pub fn recreate_render_bundle(
        &mut self,
        ctx: &GraphicsCtx,
        view_proj_bindgroup: &wgpu::BindGroup,
        lights: &LightsBuffer,
    ) {
        self.render_bundle = create_models_render_bundle(
            &self.pipeline,
            &self.models,
            &self.materials,
            &self.atlas,
            ctx,
            view_proj_bindgroup,
            lights,
        );
    }
}

fn create_models_render_bundle(
    pipeline: &wgpu::RenderPipeline,
    models: &ModelsBuffer,
    materials: &MaterialsBuffer,
    atlas: &AtlasUniform,

    ctx: &GraphicsCtx,
    view_proj_bindgroup: &wgpu::BindGroup,
    lights: &LightsBuffer,
) -> RenderBundle {
    let mut encoder =
        ctx.device
            .create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                label: None,
                color_formats: &[Some(ctx.surface_format)],
                depth_stencil: Some(RenderBundleDepthStencil {
                    format: TextureWrapper::DEPTH_FORMAT,
                    depth_read_only: false,
                    stencil_read_only: true,
                }),
                sample_count: 1,
                multiview: None,
            });

    encoder.set_pipeline(&pipeline);
    encoder.set_bind_group(0, view_proj_bindgroup, &[]);
    encoder.set_bind_group(1, &materials.bind_group, &[]);
    encoder.set_bind_group(2, &atlas.bind_group, &[]);
    encoder.set_bind_group(3, &lights.bind_group, &[]);
    encoder.set_vertex_buffer(0, models.vertex_buffer.as_slice());
    encoder.set_vertex_buffer(1, models.instance_buffer.as_slice());
    encoder.set_index_buffer(models.index_buffer.as_slice(), wgpu::IndexFormat::Uint16);
    encoder.draw_indexed_indirect(&models.indirect_buffer.inner(), 0);

    encoder.finish(&wgpu::RenderBundleDescriptor {
        label: Some("model_render_bundle"),
    })
}

const DEFAULT_SINGLE_INSTANCE: &[ModelInstance] = &[ModelInstance {
    transform: [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ],
    material_id: 0,
}];

const STRESS_TEST_INSTANCES: LazyCell<Vec<ModelInstance>> = LazyCell::new(|| {
    iter_3d(-25..25, -5..6, -50..0)
        .map(|(x, y, z)| {
            ModelInstance::new(
                Matrix4::new_translation(&Vector3::new(
                    x as f32 * 5.,
                    y as f32 * 5.,
                    z as f32 * 5.,
                )),
                0,
            )
        })
        .collect::<Vec<_>>()
});

fn load_test_model() -> EntityModel {
    let obj_text = include_str!("../../../assets/Astronaut.obj");
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);
    let (models, mat_res) = tobj::load_obj_buf(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| {
            println!("Want to load material: {p:?}");
            Ok(Default::default())
        },
    )
    .expect("Failed to load model");
    //let materials = mat_res.expect("Failed to load materials");

    EntityModel {
        meshes: models.into_iter().map(|m| m.mesh).collect(),
    }
}
