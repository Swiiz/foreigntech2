use camera::{view_proj_bindgroup, Projection, View};
use ctx::{Frame, GraphicsCtx};

pub use egui::FullOutput as EguiOutput;
pub use egui_wgpu::Renderer as EguiRenderer;
use egui_wgpu::ScreenDescriptor;
use model::renderer::ModelRenderer;
use nalgebra::Matrix4;
use utils::{TextureWrapper, UniformBuffer};

pub mod atlas;
pub mod camera;
pub mod color;
pub mod ctx;
pub mod light;
pub mod model;
pub mod utils;

pub struct GlobalRenderer {
    egui: EguiRenderer,
    models: ModelRenderer,

    view: UniformBuffer<Matrix4<f32>>,
    proj: UniformBuffer<Matrix4<f32>>,

    depth_texture: TextureWrapper,
}

pub struct RenderData {
    pub window_size: (u32, u32),
    pub aspect_ratio: f32,

    pub egui_ctx: egui::Context,
    pub egui_output: EguiOutput,
}

impl GlobalRenderer {
    pub fn new(ctx: &GraphicsCtx) -> Self {
        let view = UniformBuffer::new("view", ctx, &Matrix4::identity());
        let proj = UniformBuffer::new("camera", ctx, &Matrix4::identity());
        let view_proj_bindgroup = view_proj_bindgroup(ctx, &view, &proj);

        let depth_texture = TextureWrapper::new_depth("3d", ctx, ctx.viewport_size);

        let egui = EguiRenderer::new(
            &ctx.device,
            ctx.surface_format,
            Some(TextureWrapper::DEPTH_FORMAT),
            1,
            false,
        );
        let models = ModelRenderer::new(ctx, &view_proj_bindgroup);

        Self {
            egui,
            models,
            view,
            proj,
            depth_texture,
        }
    }

    pub fn update_viewport_size(&mut self, ctx: &GraphicsCtx) {
        self.depth_texture = TextureWrapper::new_depth("3d", ctx, ctx.viewport_size);
    }

    pub fn update_view(&self, ctx: &GraphicsCtx, view: &View) -> () {
        self.view.update(ctx, &view.compute_matrix());
    }

    pub fn update_proj(&self, ctx: &GraphicsCtx, proj: &Projection) -> () {
        self.proj.update(ctx, &proj.compute_matrix());
    }

    pub fn submit(&mut self, ctx: &GraphicsCtx, render_state: RenderData) {
        if let Some(mut frame) = ctx.next_frame() {
            let mut render_pass =
                clear_color_render_pass(&mut frame, Some(&self.depth_texture)).forget_lifetime();

            render_pass.execute_bundles([&self.models.render_bundle]);

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
