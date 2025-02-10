use nalgebra::{Point3, Vector3};

use super::color::Color3;

pub struct LightsBuffer {
    storage_buffer: wgpu::Buffer,
    count_uniform: super::UniformBuffer<u32>,
    pub bind_group: wgpu::BindGroup,
}

impl LightsBuffer {
    pub fn new(ctx: &super::GraphicsCtx, lights: &[RawLight]) -> Self {
        let storage_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Lights Storage Buffer"),
                contents: bytemuck::cast_slice(lights),
                usage: wgpu::BufferUsages::STORAGE,
            },
        );

        let count_uniform = super::UniformBuffer::new("lights_count", ctx, &(lights.len() as u32));

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &lights_buffer_bind_group_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: storage_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: count_uniform.inner.as_entire_binding(),
                },
            ],
            label: Some("Lights Bind Group"),
        });

        Self {
            storage_buffer,
            count_uniform,
            bind_group,
        }
    }
}

pub fn lights_buffer_bind_group_layout(ctx: &super::GraphicsCtx) -> wgpu::BindGroupLayout {
    ctx.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
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
            label: Some("Lights Bind Group Layout"),
        })
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawLight {
    pub position: [f32; 3],
    intensity: f32,
    pub direction: [f32; 3],
    pub cut_off: f32,
    pub color: [f32; 3],
    pub light_type: u32, // 0 = Point, 1 = Directional, 2 = Spotlight
}

pub enum Light {
    Point {
        color: Color3,
        intensity: f32,
        position: Point3<f32>,
    },
    Directional {
        color: Color3,
        intensity: f32,
        direction: Vector3<f32>,
    },
    Spotlight {
        color: Color3,
        intensity: f32,
        position: Point3<f32>,
        direction: Vector3<f32>,
        cut_off: f32,
    },
}

impl Into<RawLight> for Light {
    fn into(self) -> RawLight {
        match self {
            Light::Point {
                position,
                color,
                intensity,
            } => RawLight {
                position: position.into(),
                intensity,
                direction: [0.0, 0.0, 0.0],
                color: color.into(),
                cut_off: 0.0,
                light_type: 0,
            },
            Light::Directional {
                direction,
                color,
                intensity,
            } => RawLight {
                position: [0.0, 0.0, 0.0],
                intensity,
                direction: direction.into(),
                color: color.into(),
                cut_off: 0.0,
                light_type: 1,
            },
            Light::Spotlight {
                position,
                direction,
                color,
                cut_off,
                intensity,
            } => RawLight {
                position: position.into(),
                intensity,
                direction: direction.into(),
                color: color.into(),
                cut_off,
                light_type: 2,
            },
        }
    }
}
