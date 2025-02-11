use std::{
    borrow::Borrow,
    collections::VecDeque,
    ops::{Deref, DerefMut},
};

use crate::utils::IdAllocator;

use super::ctx::GraphicsCtx;

pub trait CommonBuffer: Sized {
    type Item;
    fn inner(&self) -> &wgpu::Buffer;

    fn new(label: &str, ctx: &GraphicsCtx, data: &Self::Item) -> Self;
    fn new_const(label: &str, ctx: &GraphicsCtx, data: &Self::Item) -> Self;
    fn new_array(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>) -> Self;
    fn new_const_array(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>) -> Self;
    fn new_vec(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>) -> Growable<Self> {
        let data = data.borrow();
        Self::new_vec_with_capacity(label, ctx, data, data.len())
    }
    fn new_vec_with_capacity(
        label: &str,
        ctx: &GraphicsCtx,
        data: impl Borrow<[Self::Item]>,
        capacity: usize,
    ) -> Growable<Self>;
    fn new_empty(label: &str, ctx: &GraphicsCtx, capacity: usize) -> Self;
    fn new_empty_vec(label: &str, ctx: &GraphicsCtx, capacity: usize) -> Growable<Self>;

    fn binding(&self) -> wgpu::BindingResource<'_> {
        self.inner().as_entire_binding()
    }
    fn as_slice(&self) -> wgpu::BufferSlice<'_> {
        self.inner().slice(..)
    }
}

/// Equivalent to the wgpu::BufferUsages::COPY_DST flag
pub trait WriteBuffer {
    type Item;

    fn write_array_at_index(&self, ctx: &GraphicsCtx, data: &impl Borrow<[Self::Item]>, index: u32);
    fn write_at_index(&self, ctx: &GraphicsCtx, data: &Self::Item, index: u32);

    fn write(&self, ctx: &GraphicsCtx, data: &Self::Item) {
        self.write_at_index(ctx, data, 0);
    }
    fn write_array(&self, ctx: &GraphicsCtx, data: &impl Borrow<[Self::Item]>) {
        self.write_array_at_index(ctx, data, 0);
    }
}

macro_rules! impl_buffer_write {
    ($($name:ident : $usage:ident),*) => {
        $(
          pub struct $name<T> {
              inner: wgpu::Buffer,
              _marker: std::marker::PhantomData<T>,
          }

          impl<T: bytemuck::NoUninit> CommonBuffer for $name<T> {
                type Item = T;

                fn inner(&self) -> &wgpu::Buffer {
                    &self.inner
                }

                fn new(label: &str, ctx: &GraphicsCtx, data: &Self::Item) -> Self {
                    Self::new_array(label, ctx, [*data])
                }

                fn new_const(label: &str, ctx: &GraphicsCtx, data: &Self::Item) -> Self {
                    Self::new_const_array(label, ctx, [*data])
                }

               fn new_array(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>) -> Self {
                  let buffer = wgpu::util::DeviceExt::create_buffer_init(
                      &ctx.device,
                      &wgpu::util::BufferInitDescriptor {
                          label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
                          contents: bytemuck::cast_slice(data.borrow()),
                          usage: wgpu::BufferUsages::$usage | wgpu::BufferUsages::COPY_DST,
                      },
                  );
                  Self {
                      inner: buffer,
                      _marker: std::marker::PhantomData,
                  }
              }

               fn new_const_array(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>) -> Self {
                  let buffer = wgpu::util::DeviceExt::create_buffer_init(
                      &ctx.device,
                      &wgpu::util::BufferInitDescriptor {
                          label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
                          contents: bytemuck::cast_slice(data.borrow()),
                          usage: wgpu::BufferUsages::$usage,
                      },
                  );
                  Self {
                      inner: buffer,
                      _marker: std::marker::PhantomData,
                  }
              }

               fn new_vec_with_capacity(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>, capacity: usize) -> Growable<Self>  {
                    let slice = data.borrow();
                    let label = format!("{} Buffer: {}", stringify!($name), label);
                    if capacity < slice.len() {
                        panic!("Growable (vec) buffer capacity must be greater than or equal to the length of the provided data slice")
                    }
                    let buffer = wgpu::util::DeviceExt::create_buffer_init(
                        &ctx.device,
                        &wgpu::util::BufferInitDescriptor {
                            label: Some(&label),
                            contents: bytemuck::cast_slice(slice),
                            usage: wgpu::BufferUsages::$usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                        },
                    );
                    Growable {
                        inner: Self {
                            inner: buffer,
                            _marker: std::marker::PhantomData,
                        },
                        capacity,
                        #[cfg(debug_assertions)]
                        label,
                    }
                }

                 fn new_empty(label: &str, ctx: &GraphicsCtx, capacity: usize) -> Self {
                    let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
                        size: capacity as u64 * std::mem::size_of::<T>() as u64,
                        usage: wgpu::BufferUsages::$usage | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    Self {
                        inner: buffer,
                        _marker: std::marker::PhantomData,
                    }
                }

                 fn new_empty_vec(label: &str, ctx: &GraphicsCtx, capacity: usize) -> Growable<Self> {
                    let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
                        size: capacity as u64 * std::mem::size_of::<T>() as u64,
                        usage: wgpu::BufferUsages::$usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    });
                    Growable {
                        inner: Self {
                            inner: buffer,
                            _marker: std::marker::PhantomData,
                        },
                        capacity,
                        #[cfg(debug_assertions)]
                        label: label.to_string(),
                    }
                }
          }

          impl<T: bytemuck::NoUninit> WriteBuffer for $name<T> {
              type Item = T;

              fn write_at_index(&self, ctx: &GraphicsCtx, data: &T, offset: u32) {
                  ctx.queue
                      .write_buffer(&self.inner, offset as u64 * std::mem::size_of::<T>() as u64, bytemuck::cast_slice(&[*data]));
              }
              fn write_array_at_index(&self, ctx: &GraphicsCtx, data: &impl Borrow<[T]>, offset: u32) {
                  ctx.queue.write_buffer(&self.inner, offset as u64 * std::mem::size_of::<T>() as u64, bytemuck::cast_slice(data.borrow()));
              }
          }

        )*
    };
}

impl_buffer_write!(
    VertexBuffer: VERTEX,
    IndexBuffer: INDEX,
    InstanceBuffer: VERTEX,
    UniformBuffer: UNIFORM,
    StorageBuffer: STORAGE
);

pub struct IndirectBuffer {
    pub inner: wgpu::Buffer,
}

impl IndirectBuffer {
    const ARG_INSTANCE_COUNT_BYTE_OFFSET: u64 = 4;
    const ARG_FIRST_INSTANCE_BYTE_OFFSET: u64 = 16;

    pub fn write_instance_count_at_index(
        &self,
        ctx: &GraphicsCtx,
        index: u32,
        instance_count: u32,
    ) {
        ctx.queue.write_buffer(
            &self.inner,
            Self::ARG_INSTANCE_COUNT_BYTE_OFFSET
                + index as u64 * Self::ARG_FIRST_INSTANCE_BYTE_OFFSET,
            bytemuck::bytes_of(&instance_count),
        );
    }

    pub fn write_first_instance_at_index(
        &self,
        ctx: &GraphicsCtx,
        index: u32,
        first_instance: u32,
    ) {
        ctx.queue.write_buffer(
            &self.inner,
            Self::ARG_FIRST_INSTANCE_BYTE_OFFSET
                + index as u64 * Self::ARG_FIRST_INSTANCE_BYTE_OFFSET,
            bytemuck::bytes_of(&first_instance),
        );
    }
}

impl CommonBuffer for IndirectBuffer {
    type Item = wgpu::util::DrawIndexedIndirectArgs;

    fn inner(&self) -> &wgpu::Buffer {
        &self.inner
    }

    fn new(label: &str, ctx: &GraphicsCtx, data: &Self::Item) -> Self {
        Self::new_array(label, ctx, [*data])
    }

    fn new_const(label: &str, ctx: &GraphicsCtx, data: &Self::Item) -> Self {
        Self::new_const_array(label, ctx, [*data])
    }

    fn new_array(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>) -> Self {
        let inner = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Indirect Buffer: {}", label)),
                contents: cast_iia(data.borrow()),
                usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            },
        );

        Self { inner }
    }

    fn new_const_array(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>) -> Self {
        let inner = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Indirect Buffer: {}", label)),
                // SAFETY: `DrawIndexedIndirectArgs` is repr(C) and made to be casted to `[u32; _]`
                contents: cast_iia(data.borrow()),
                usage: wgpu::BufferUsages::INDIRECT,
            },
        );

        Self { inner }
    }

    fn new_empty(label: &str, ctx: &GraphicsCtx, capacity: usize) -> Self {
        let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
            size: capacity as u64 * std::mem::size_of::<Self::Item>() as u64,
            usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { inner: buffer }
    }

    fn new_vec_with_capacity(
        label: &str,
        ctx: &GraphicsCtx,
        data: impl Borrow<[Self::Item]>,
        capacity: usize,
    ) -> Growable<Self> {
        let slice = data.borrow();
        let label = format!("{} Buffer: {}", stringify!($name), label);
        if capacity < slice.len() {
            panic!("Growable (vec) buffer capacity must be greater than or equal to the length of the provided data slice")
        }
        let buffer = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some(&label),
                contents: cast_iia(slice),
                usage: wgpu::BufferUsages::INDIRECT
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            },
        );
        Growable {
            inner: Self { inner: buffer },
            capacity,
            #[cfg(debug_assertions)]
            label,
        }
    }

    fn new_empty_vec(label: &str, ctx: &GraphicsCtx, capacity: usize) -> Growable<Self> {
        let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
            size: capacity as u64 * std::mem::size_of::<Self::Item>() as u64,
            usage: wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        Growable {
            inner: Self { inner: buffer },
            capacity,
            #[cfg(debug_assertions)]
            label: label.to_string(),
        }
    }
}

impl WriteBuffer for IndirectBuffer {
    type Item = wgpu::util::DrawIndexedIndirectArgs;

    fn write_array_at_index(
        &self,
        ctx: &GraphicsCtx,
        data: &impl Borrow<[Self::Item]>,
        offset: u32,
    ) {
        ctx.queue.write_buffer(
            &self.inner,
            offset as u64 * std::mem::size_of::<Self::Item>() as u64,
            cast_iia(data.borrow()),
        );
    }

    fn write_at_index(&self, ctx: &GraphicsCtx, data: &Self::Item, offset: u32) {
        Self::write_array_at_index(self, ctx, &[*data], offset);
    }
}

fn cast_iia(args: &[wgpu::util::DrawIndexedIndirectArgs]) -> &[u8] {
    // SAFETY: `DrawIndexedIndirectArgs` is repr(C) and made to be casted to `[u32; _]`
    unsafe {
        std::slice::from_raw_parts(
            args.as_ptr().cast(),
            args.len() * std::mem::size_of::<wgpu::util::DrawIndexedIndirectArgs>(),
        )
    }
}

pub struct Growable<T> {
    pub inner: T,
    capacity: usize,

    #[cfg(debug_assertions)]
    label: String,
}

impl<T: CommonBuffer> Growable<T> {
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Grows the inner buffer to the next power of two that is greater than or equal to `required_size` if needed.
    pub fn maybe_grow(&mut self, ctx: &GraphicsCtx, required_size: usize) -> bool {
        let grow = required_size > self.capacity;
        if grow {
            // Compute new buffer size (double current size or required size)
            let new_capacity = self.capacity.max(1) * 2;
            let new_capacity = new_capacity.max(required_size);
            let new_buffer = T::new_empty_vec(
                {
                    #[cfg(debug_assertions)]
                    let l = self.label.as_str();
                    #[cfg(not(debug_assertions))]
                    let l = "";
                    l
                },
                ctx,
                new_capacity,
            );

            if self.capacity > 0 {
                let mut encoder =
                    ctx.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Growable Buffer Copy Encoder"),
                        });
                encoder.copy_buffer_to_buffer(
                    &self.inner(),
                    0,
                    &new_buffer.inner(),
                    0,
                    self.capacity as u64 * std::mem::size_of::<T>() as u64,
                );
                ctx.queue.submit(Some(encoder.finish()));
            }

            *self = new_buffer;
        }
        return grow;
    }

    pub fn maybe_grow_around(
        &mut self,
        ctx: &GraphicsCtx,
        index: u32,
        required_size: usize,
    ) -> bool {
        let grow = required_size > self.capacity;
        if grow {
            // Compute new buffer size (double current size or required size)
            let new_capacity = self.capacity.max(1) * 2;
            let new_capacity = new_capacity.max(required_size);
            let new_buffer = T::new_empty_vec(
                {
                    #[cfg(debug_assertions)]
                    let l = self.label.as_str();
                    #[cfg(not(debug_assertions))]
                    let l = "";
                    l
                },
                ctx,
                new_capacity,
            );

            if self.capacity > 0 {
                let index_offset = index as u64 * std::mem::size_of::<T::Item>() as u64;

                let mut encoder =
                    ctx.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Growable Buffer Copy Encoder"),
                        });
                encoder.copy_buffer_to_buffer(
                    &self.inner(),
                    0,
                    &new_buffer.inner(),
                    0,
                    index_offset,
                );
                encoder.copy_buffer_to_buffer(
                    &self.inner(),
                    index_offset,
                    &new_buffer.inner(),
                    (index as u64 + (required_size - self.capacity) as u64)
                        * std::mem::size_of::<T::Item>() as u64,
                    (self.capacity as u64 - index as u64) * std::mem::size_of::<T::Item>() as u64,
                );
                ctx.queue.submit(Some(encoder.finish()));
            }

            *self = new_buffer;
        }
        return grow;
    }
}

impl<T> Deref for Growable<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Growable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct Mapped<T: CommonBuffer> {
    pub inner: Growable<T>,
    pub changes: Vec<(u32, T::Item)>,

    ids: IdAllocator,
}

impl<I: Default, T: CommonBuffer<Item = I> + WriteBuffer<Item = I>> Mapped<T> {
    pub fn new(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[I]>) -> Self {
        let data = data.borrow();
        let inner = T::new_vec(label, ctx, data);
        Self {
            inner,
            changes: vec![],
            ids: IdAllocator::new_packed(data.len() as u32),
        }
    }

    pub fn push(&mut self, data: I) -> u32 {
        let idx = self.ids.allocate();
        self.changes.push((idx, data));
        idx
    }

    pub fn set(&mut self, idx: u32, data: I) {
        if idx >= self.ids.len() {
            panic!("Index out of bounds");
        }
        self.changes.push((idx, data));
    }

    pub fn remove(&mut self, idx: u32) {
        self.set(idx, Default::default());
        self.ids.free(idx);
    }

    pub fn len(&self) -> u32 {
        self.ids.len()
    }

    //TODO: use staging belt?
    /// Returns true if the buffer was grown
    pub fn apply_changes(&mut self, ctx: &GraphicsCtx) -> bool {
        let grown = self.inner.maybe_grow(ctx, self.ids.len() as usize);
        for (idx, data) in self.changes.drain(..) {
            self.inner.write_at_index(ctx, &data, idx);
        }
        grown
    }
}

impl<T: CommonBuffer> Deref for Mapped<T> {
    type Target = Growable<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: CommonBuffer> DerefMut for Mapped<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
