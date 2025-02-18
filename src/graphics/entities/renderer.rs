use nalgebra::{Matrix4, Vector3};
use nd_iter::iter_3d;
use wgpu::{include_wgsl, DepthStencilState};

use crate::graphics::{
    atlas::{atlas_uniform_bind_group_layout, AtlasPacker, AtlasUniform},
    buffer::CommonBuffer,
    camera::{view_proj_bind_group_layout, CameraUniform},
    ctx::GraphicsCtx,
    entities::model::materials_buffer_bind_group_layout,
    light::{lights_buffer_bind_group_layout, LightsUniform},
    utils::TextureWrapper,
};

use super::model::{load_model, MaterialsBuffer, ModelInstance, ModelVertex, ModelsBuffer};

pub struct EntitiesRenderer {
    pub models: ModelsBuffer,
    pub materials: MaterialsBuffer,
    pub atlas: AtlasUniform,

    pipeline: wgpu::RenderPipeline,
}

impl EntitiesRenderer {
    pub fn new(ctx: &GraphicsCtx) -> Self {
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
                    buffers: &[ModelVertex::buffer_desc(), ModelInstance::buffer_desc()],
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

        let astronaut = load_model("Astronaut");
        let earth = load_model("Earth");

        let materials = [astronaut.materials, earth.materials].concat();
        let textures = [astronaut.textures, earth.textures].concat();
        let entities = [
            (&astronaut.meshes, vec![single_instance(0)]),
            (
                &earth.meshes,
                vec![stress_test_instances(1), stress_test_instances(2)],
            ),
        ];

        let models = ModelsBuffer::new(ctx, entities);
        let materials = MaterialsBuffer::new(ctx, &materials);
        let atlas = AtlasPacker::from_textures(textures).build_atlas(ctx);

        Self {
            models,
            materials,
            atlas,
            pipeline,
        }
    }

    pub fn render(
        &mut self,
        render_pass: &mut wgpu::RenderPass<'static>,
        camera: &CameraUniform,
        lights: &LightsUniform,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &camera.view_proj_bindgroup, &[]);
        render_pass.set_bind_group(1, &self.materials.bind_group, &[]);
        render_pass.set_bind_group(2, &self.atlas.bind_group, &[]);
        render_pass.set_bind_group(3, &lights.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.models.vertex_buffer.as_slice());
        render_pass.set_vertex_buffer(1, self.models.instance_buffer.as_slice());
        render_pass.set_index_buffer(
            self.models.index_buffer.as_slice(),
            wgpu::IndexFormat::Uint16,
        );
        render_pass.multi_draw_indexed_indirect(
            &self.models.indirect_buffer.inner(),
            0,
            self.models.mesh_count(),
        );
    }

    pub fn apply_changes(&mut self, ctx: &GraphicsCtx) {
        self.models.apply_changes(ctx);
    }
}

fn single_instance(material_id: u32) -> Vec<ModelInstance> {
    vec![ModelInstance {
        transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        material_id: material_id,
    }]
}

fn stress_test_instances(material_id: u32) -> Vec<ModelInstance> {
    iter_3d(-25..25, -5..6, -50..0)
        .map(|(x, y, z)| {
            ModelInstance::new(
                Matrix4::new_translation(&Vector3::new(
                    x as f32 * 5.,
                    y as f32 * 5.,
                    z as f32 * 5.,
                )),
                material_id,
            )
        })
        .collect::<Vec<_>>()
}
