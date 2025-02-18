use nalgebra::{Matrix4, Perspective3, Point3, Rotation3, Vector3, Vector4};

use crate::constants;

use super::{
    buffer::{CommonBuffer, UniformBuffer, WriteBuffer},
    ctx::GraphicsCtx,
    utils::fovy,
};

const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, 1.0,
);

pub struct Camera {
    pub eye: Point3<f32>,
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub roll_deg: f32,
    pub up: Vector3<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: Point3::new(0.0, 1.0, 2.0),
            pitch_deg: 0.0,
            yaw_deg: 0.0,
            roll_deg: 0.0,
            up: Vector3::new(0.0, 1.0, 0.0),
        }
    }
}

impl Camera {
    pub fn compute_view_matrix(&self) -> Matrix4<f32> {
        self.compute_rot_matrix() * Matrix4::new_translation(&-Vector4::from(self.eye).xyz())
    }

    pub fn compute_rot_matrix(&self) -> Matrix4<f32> {
        (Rotation3::from_axis_angle(&Vector3::x_axis(), -self.pitch_deg.to_radians())
            * Rotation3::from_axis_angle(&Vector3::y_axis(), -self.yaw_deg.to_radians())
            * Rotation3::from_axis_angle(&Vector3::z_axis(), -self.roll_deg.to_radians()))
        .to_homogeneous()
    }
}

pub struct Projection {
    pub aspect_ratio: f32,
    pub fov_deg: f32,
}

impl Projection {
    pub fn compute_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX
            * Perspective3::new(
                self.aspect_ratio,
                fovy(self.fov_deg, self.aspect_ratio),
                constants::MODEL_ZNEAR,
                constants::MODE_ZFAR,
            )
            .to_homogeneous()
    }
}

pub fn view_proj_bind_group_layout(ctx: &GraphicsCtx) -> wgpu::BindGroupLayout {
    ctx.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("view_proj_bind_group_layout"),
        })
}

pub fn inv_view_proj_bind_group_layout(ctx: &GraphicsCtx) -> wgpu::BindGroupLayout {
    ctx.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("inv_view_proj_bind_group_layout"),
        })
}

pub fn view_proj_bindgroup(
    ctx: &GraphicsCtx,
    view_buffer: &UniformBuffer<Matrix4<f32>>,
    proj_buffer: &UniformBuffer<Matrix4<f32>>,
) -> wgpu::BindGroup {
    ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &view_proj_bind_group_layout(ctx),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: view_buffer.binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: proj_buffer.binding(),
            },
        ],
        label: Some("view_proj_bindgroup"),
    })
}

pub struct CameraUniform {
    view: UniformBuffer<Matrix4<f32>>,
    proj: UniformBuffer<Matrix4<f32>>,
    inv_view: UniformBuffer<Matrix4<f32>>,
    inv_proj: UniformBuffer<Matrix4<f32>>,
    pub view_proj_bindgroup: wgpu::BindGroup,
    pub inv_view_proj_bindgroup: wgpu::BindGroup,
}

impl CameraUniform {
    pub fn new(ctx: &GraphicsCtx) -> Self {
        let view_buffer = UniformBuffer::new("view", ctx, &Matrix4::identity());
        let proj_buffer = UniformBuffer::new("camera", ctx, &Matrix4::identity());
        let view_proj_bindgroup = view_proj_bindgroup(ctx, &view_buffer, &proj_buffer);

        let inv_view_buffer = UniformBuffer::new("inv_view", ctx, &Matrix4::identity());
        let inv_proj_buffer = UniformBuffer::new("inv_camera", ctx, &Matrix4::identity());
        let inv_view_proj_bindgroup =
            self::view_proj_bindgroup(ctx, &inv_view_buffer, &inv_proj_buffer);

        Self {
            view: view_buffer,
            proj: proj_buffer,
            inv_view: inv_view_buffer,
            inv_proj: inv_proj_buffer,
            view_proj_bindgroup,
            inv_view_proj_bindgroup,
        }
    }

    pub fn update_view(&mut self, ctx: &GraphicsCtx, camera: &Camera) {
        let view = camera.compute_view_matrix();
        self.view.write(ctx, &view);
        self.inv_view.write(
            ctx,
            &view.try_inverse().expect("View matrix is not invertible"),
        );
    }

    pub fn update_proj(&mut self, ctx: &GraphicsCtx, proj: &Projection) {
        let proj = proj.compute_matrix();
        self.proj.write(ctx, &proj);
        self.inv_proj.write(
            ctx,
            &proj
                .try_inverse()
                .expect("Projection matrix is not invertible"),
        );
    }
}
