use std::cell::LazyCell;

use buffer::{CommonBuffer, UniformBuffer, WriteBuffer};
use camera::{Camera, CameraUniform};
use color::Color3;
use ctx::{Frame, GraphicsCtx};

pub use egui::FullOutput as EguiOutput;
pub use egui_wgpu::Renderer as EguiRenderer;
use egui_wgpu::ScreenDescriptor;
use entities::renderer::EntitiesRenderer;
use light::{Light, LightsUniform, RawLight};
use nalgebra::{Matrix4, Point3, Vector3};
use terrain::TerrainRenderer;
use utils::TextureWrapper;

pub mod assets;
pub mod atlas;
pub mod buffer;
pub mod camera;
pub mod color;
pub mod ctx;
pub mod entities;
pub mod light;
pub mod terrain;
pub mod utils;

pub struct GlobalRenderer {
    egui: EguiRenderer,
    pub terrain: TerrainRenderer,
    pub entities: EntitiesRenderer,

    pub lights: LightsUniform,
    pub camera: CameraUniform,

    depth_texture: TextureWrapper,
}

pub struct RenderData {
    pub window_size: (u32, u32),
    pub aspect_ratio: f32,

    pub egui_ctx: egui::Context,
    pub egui_output: EguiOutput,
}

const TEST_LIGHTS: LazyCell<[RawLight; 3]> = LazyCell::new(|| {
    [
        Light::Directional {
            direction: Vector3::new(0.0, -0.9, -0.3).normalize(),
            intensity: 1.5,
            color: Color3::WHITE,
        }
        .into(),
        Light::Point {
            position: Point3::new(5.0, 5.0, 1.0),
            intensity: 5.0,
            color: Color3::CYAN,
        }
        .into(),
        Light::Point {
            position: Point3::new(-5.0, 1.0, 1.0),
            intensity: 5.0,
            color: Color3::RED,
        }
        .into(),
    ]
});

impl GlobalRenderer {
    pub fn new(ctx: &GraphicsCtx) -> Self {
        let lights = LightsUniform::new(ctx, TEST_LIGHTS.as_ref());
        let camera = CameraUniform::new(ctx);

        let depth_texture = TextureWrapper::new_depth("3d", ctx, ctx.viewport_size);

        let egui = EguiRenderer::new(
            &ctx.device,
            ctx.surface_format,
            Some(TextureWrapper::DEPTH_FORMAT),
            1,
            false,
        );

        let entities = EntitiesRenderer::new(ctx);
        let terrain = TerrainRenderer::new(ctx, &camera);

        Self {
            egui,
            entities,
            terrain,
            lights,
            camera,
            depth_texture,
        }
    }

    pub fn update_viewport_size(&mut self, ctx: &GraphicsCtx) {
        self.depth_texture = TextureWrapper::new_depth("3d", ctx, ctx.viewport_size);
    }

    pub fn submit(&mut self, ctx: &GraphicsCtx, render_state: RenderData) {
        self.lights.apply_changes(ctx);
        self.entities.apply_changes(ctx);

        if let Some(mut frame) = ctx.next_frame() {
            let mut render_pass =
                clear_color_render_pass(&mut frame, Some(&self.depth_texture)).forget_lifetime();

            render_pass.execute_bundles([&self.terrain.render_bundle]);
            self.entities
                .render(&mut render_pass, &self.camera, &self.lights);

            render_egui(
                &mut self.egui,
                ctx,
                &mut frame,
                &mut render_pass,
                ScreenDescriptor {
                    size_in_pixels: render_state.window_size.into(),
                    pixels_per_point: render_state.aspect_ratio,
                },
                &render_state.egui_ctx,
                render_state.egui_output,
            );

            drop(render_pass);

            frame.present(ctx);
        }
    }
}

fn clear_color_render_pass<'a>(
    r: &'a mut Frame,
    depth_texture: Option<&'a TextureWrapper>,
) -> wgpu::RenderPass<'a> {
    r.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &r.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        occlusion_query_set: None,
        timestamp_writes: None,
        depth_stencil_attachment: depth_texture.map(|t: &TextureWrapper| {
            wgpu::RenderPassDepthStencilAttachment {
                view: &t.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }
        }),
    })
}

fn render_egui(
    renderer: &mut EguiRenderer,
    g: &GraphicsCtx,
    r: &mut Frame,
    pass: &mut wgpu::RenderPass<'static>,
    screen_descriptor: ScreenDescriptor,
    ctx: &egui::Context,
    output: EguiOutput,
) {
    let paint_jobs = ctx.tessellate(output.shapes, output.pixels_per_point);

    for (id, image_delta) in &output.textures_delta.set {
        renderer.update_texture(&g.device, &g.queue, *id, image_delta);
    }
    for id in &output.textures_delta.free {
        renderer.free_texture(id);
    }

    renderer.update_buffers(
        &g.device,
        &g.queue,
        &mut r.encoder,
        &paint_jobs,
        &screen_descriptor,
    );

    renderer.render(pass, &paint_jobs, &screen_descriptor);
}
