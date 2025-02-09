use crate::graphics::GraphicsCtx;

pub struct UniformBuffer<T> {
    pub inner: wgpu::Buffer,
    _marker: std::marker::PhantomData<T>,
}

impl<T: bytemuck::NoUninit> UniformBuffer<T> {
    pub fn new(label: &str, ctx: &GraphicsCtx, data: &T) -> Self {
        let buffer = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Uniform Buffer: {}", label)),
                contents: bytemuck::cast_slice(&[*data]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );
        Self {
            inner: buffer,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn update(&self, ctx: &GraphicsCtx, data: &T) -> () {
        ctx.queue
            .write_buffer(&self.inner, 0, bytemuck::cast_slice(&[*data]));
    }
}

pub struct TextureWrapper {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl TextureWrapper {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new_2d(
        label: &str,
        ctx: &GraphicsCtx,
        (width, height): (u32, u32),
        component_count: u32,
        data: &[u8],
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1, //TODO: mipmaps
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(&format!("Diffuse Texture: {}", label)),
            view_formats: &[ctx.surface_format],
        });

        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(component_count * width),
                rows_per_image: Some(height),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("Texture Sampler: {}", label)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view: texture_view,
            sampler,
        }
    }

    pub fn new_depth(label: &str, ctx: &GraphicsCtx, (width, height): (u32, u32)) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let label = format!("Depth Texture: {}", label);
        let desc = wgpu::TextureDescriptor {
            label: Some(&label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = ctx.device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("Depth Sampler: {}", label)),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
        }
    }
}
