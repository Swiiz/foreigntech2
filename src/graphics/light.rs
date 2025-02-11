use nalgebra::{Point3, Vector3};

use super::{
    buffer::{CommonBuffer, Mapped, StorageBuffer, WriteBuffer},
    color::Color3,
};

pub struct LightsBuffer {
    pub storage_buffer: Mapped<StorageBuffer<RawLight>>,
    count_uniform: super::UniformBuffer<u32>,
    pub bind_group: wgpu::BindGroup,
}

impl LightsBuffer {
    pub fn new(ctx: &super::GraphicsCtx, lights: &[RawLight]) -> Self {
        let storage_buffer = Mapped::<StorageBuffer<_>>::new("Lights", ctx, lights);
        let count_uniform = super::UniformBuffer::new("lights_count", ctx, &(lights.len() as u32));

        let bind_group = lights_buffer_bindgroup(ctx, &(**storage_buffer), &count_uniform);

        Self {
            storage_buffer,
            count_uniform,
            bind_group,
        }
    }

    /// Returns true if the bindgroup was recreated, thus requiring the renderbundle to be recreated
    pub fn apply_changes(&mut self, ctx: &super::GraphicsCtx) -> bool {
        let grown = self.storage_buffer.apply_changes(ctx);
        if grown {
            self.bind_group =
                lights_buffer_bindgroup(ctx, &(**self.storage_buffer), &self.count_uniform)
        }
        self.count_uniform
            .write(ctx, &(self.storage_buffer.len() as u32));
        return grown;
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

fn lights_buffer_bindgroup(
    ctx: &super::GraphicsCtx,
    storage: &impl CommonBuffer,
    count: &impl CommonBuffer,
) -> wgpu::BindGroup {
    ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &lights_buffer_bind_group_layout(ctx),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: storage.binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: count.binding(),
            },
        ],
        label: Some("Lights Bind Group"),
    })
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct RawLight {
    pub position: [f32; 3],
    intensity: f32,
    pub direction: [f32; 3],
    pub cut_off: f32,
    pub color: [f32; 3],
    pub light_type: u32, // 0 = None, 1 = Point, 2 = Directional, 3 = Spotlight
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum Light {
    #[default]
    None,
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
    //Todo: fix spotlight in shader
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
            Light::None => RawLight::default(),
            Light::Point {
                position,
                color,
                intensity,
            } => RawLight {
                position: position.into(),
                intensity,
                color: color.into(),
                light_type: 1,
                ..Default::default()
            },
            Light::Directional {
                direction,
                color,
                intensity,
            } => RawLight {
                intensity,
                direction: direction.into(),
                color: color.into(),
                light_type: 2,
                ..Default::default()
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
                light_type: 3,
            },
        }
    }
}

impl Light {
    pub fn default_point() -> Self {
        Self::Point {
            color: Color3::WHITE,
            intensity: 1.0,
            position: Point3::new(0.0, 0.0, 0.0),
        }
    }

    pub fn default_directional() -> Self {
        Self::Directional {
            color: Color3::WHITE,
            intensity: 1.0,
            direction: Vector3::new(0.0, -0.9, -0.3).normalize(),
        }
    }

    pub fn default_spotlight() -> Self {
        Self::Spotlight {
            color: Color3::WHITE,
            intensity: 1.0,
            position: Point3::new(0.0, 0.0, 0.0),
            direction: Vector3::new(0.0, -0.9, -0.3).normalize(),
            cut_off: 20.0,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Light::None => "None",
            Light::Point { .. } => "Point",
            Light::Directional { .. } => "Directional",
            Light::Spotlight { .. } => "Spotlight",
        }
    }
}
