use std::{
    borrow::Borrow,
    ops::{Deref, DerefMut},
};

use bytemuck::NoUninit;

use crate::utils::{DenseArrayOp, DenseId, DenseIdAllocator, SparseIdAllocator};

use super::ctx::GraphicsCtx;

pub trait CommonBuffer: Sized {
    type Item;
    const ITEM_BYTE_SIZE: u64 = std::mem::size_of::<Self::Item>() as u64;

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
pub trait WriteBuffer: CommonBuffer {
    fn write_array_at_index(&self, ctx: &GraphicsCtx, data: &impl Borrow<[Self::Item]>, index: u32);
    fn write_at_index(&self, ctx: &GraphicsCtx, data: &Self::Item, index: u32);

    fn write(&self, ctx: &GraphicsCtx, data: &Self::Item) {
        self.write_at_index(ctx, data, 0);
    }
    fn write_array(&self, ctx: &GraphicsCtx, data: &impl Borrow<[Self::Item]>) {
        self.write_array_at_index(ctx, data, 0);
    }

    fn swap_at_indices(&self, ctx: &GraphicsCtx, a: u32, b: u32)
    where
        Self::Item: bytemuck::NoUninit,
    {
        let staging_buffer = StagingBuffer::<Self::Item>::new_empty("Swap", ctx, 2);
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Swap at indices"),
            });
        let item_size = Self::ITEM_BYTE_SIZE;

        // Copy from Self.A to Staging.A
        encoder.copy_buffer_to_buffer(
            self.inner(),
            item_size * a as u64,
            staging_buffer.inner(),
            0,
            item_size,
        );

        // Copy from Self.B to Staging.B
        encoder.copy_buffer_to_buffer(
            self.inner(),
            item_size * b as u64,
            staging_buffer.inner(),
            item_size,
            item_size,
        );

        // Copy from Staging.A to Self.B
        encoder.copy_buffer_to_buffer(
            staging_buffer.inner(),
            0,
            self.inner(),
            item_size * b as u64,
            item_size,
        );

        // Copy from Staging.B to Self.A
        encoder.copy_buffer_to_buffer(
            staging_buffer.inner(),
            item_size,
            self.inner(),
            item_size * a as u64,
            item_size,
        );

        ctx.queue.submit(Some(encoder.finish()));
    }
}

macro_rules! impl_buffer_write {
    ($($name:ident : $($usage:ident)?),*) => {
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
                          usage: $(wgpu::BufferUsages::$usage |)? wgpu::BufferUsages::COPY_DST,
                      },
                  );
                  Self {
                      inner: buffer,
                      _marker: std::marker::PhantomData,
                  }
              }

                #[allow(unreachable_code)]
               fn new_const_array(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>) -> Self {
                    $(
                        let buffer = wgpu::util::DeviceExt::create_buffer_init(
                            &ctx.device,
                            &wgpu::util::BufferInitDescriptor {
                                label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
                                contents: bytemuck::cast_slice(data.borrow()),
                                usage: wgpu::BufferUsages::$usage,
                            },
                        );
                        return Self {
                            inner: buffer,
                            _marker: std::marker::PhantomData,
                        };
                    )?

                    unimplemented!("Staging buffer cannot be constant");
              }

               fn new_vec_with_capacity(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[Self::Item]>, capacity: usize) -> Growable<Self>  {
                    let slice = data.borrow();
                    if capacity < slice.len() {
                        panic!("Growable (vec) buffer capacity must be greater than or equal to the length of the provided data slice")
                    }
                    let buffer = wgpu::util::DeviceExt::create_buffer_init(
                        &ctx.device,
                        &wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
                            contents: bytemuck::cast_slice(slice),
                            usage: $(wgpu::BufferUsages::$usage |)? wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                        },
                    );
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

                 fn new_empty(label: &str, ctx: &GraphicsCtx, capacity: usize) -> Self {
                    let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
                        size: capacity as u64 * Self::ITEM_BYTE_SIZE,
                        usage: $(wgpu::BufferUsages::$usage |)? wgpu::BufferUsages::COPY_DST,
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
                        size: capacity as u64 * Self::ITEM_BYTE_SIZE,
                        usage: $(wgpu::BufferUsages::$usage |)? wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
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
              fn write_at_index(&self, ctx: &GraphicsCtx, data: &T, offset: u32) {
                  ctx.queue
                      .write_buffer(&self.inner, offset as u64 * Self::ITEM_BYTE_SIZE, bytemuck::cast_slice(&[*data]));
              }
              fn write_array_at_index(&self, ctx: &GraphicsCtx, data: &impl Borrow<[T]>, offset: u32) {
                  ctx.queue.write_buffer(&self.inner, offset as u64 * Self::ITEM_BYTE_SIZE, bytemuck::cast_slice(data.borrow()));
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
    StorageBuffer: STORAGE,
    StagingBuffer: COPY_SRC
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
            Self::ARG_INSTANCE_COUNT_BYTE_OFFSET + index as u64 * Self::ITEM_BYTE_SIZE,
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
            Self::ARG_FIRST_INSTANCE_BYTE_OFFSET + index as u64 * Self::ITEM_BYTE_SIZE,
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
            size: capacity as u64 * Self::ITEM_BYTE_SIZE,
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
        if capacity < slice.len() {
            panic!("Growable (vec) buffer capacity must be greater than or equal to the length of the provided data slice")
        }
        let buffer = wgpu::util::DeviceExt::create_buffer_init(
            &ctx.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
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
            label: label.to_string(),
        }
    }

    fn new_empty_vec(label: &str, ctx: &GraphicsCtx, capacity: usize) -> Growable<Self> {
        let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} Buffer: {}", stringify!($name), label)),
            size: capacity as u64 * Self::ITEM_BYTE_SIZE,
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
    fn write_array_at_index(
        &self,
        ctx: &GraphicsCtx,
        data: &impl Borrow<[Self::Item]>,
        offset: u32,
    ) {
        ctx.queue.write_buffer(
            &self.inner,
            offset as u64 * Self::ITEM_BYTE_SIZE,
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
            args.len() * IndirectBuffer::ITEM_BYTE_SIZE as usize,
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
                    self.capacity as u64 * T::ITEM_BYTE_SIZE,
                );
                ctx.queue.submit(Some(encoder.finish()));
            }

            *self = new_buffer;
        }
        grow
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

/// Mapped sparse buffer with T::default() in the free slots
pub struct MappedSparse<T: CommonBuffer> {
    pub inner: Growable<T>,
    pub changes: Vec<(u32, T::Item)>,

    ids: SparseIdAllocator,
}

impl<I: Default, T: CommonBuffer<Item = I> + WriteBuffer<Item = I>> MappedSparse<T> {
    pub fn new(label: &str, ctx: &GraphicsCtx, data: impl Borrow<[I]>) -> Self {
        let data = data.borrow();
        let inner = T::new_vec(label, ctx, data);
        Self {
            inner,
            changes: vec![],
            ids: SparseIdAllocator::new_packed(data.len() as u32),
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

impl<T: CommonBuffer> Deref for MappedSparse<T> {
    type Target = Growable<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: CommonBuffer> DerefMut for MappedSparse<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DenseMapped2d<T: CommonBuffer> {
    inner: Growable<T>,
    columns: Vec<ColumnMeta<T::Item>>,

    ttl_capacity: usize,

    #[cfg(debug_assertions)]
    label: String,
}

struct ColumnMeta<T> {
    capacity: usize,
    index_offset: usize,
    changes: Vec<ColumnOp<T>>,
    ids: DenseIdAllocator,
}

enum ColumnOp<T> {
    Insert(T, DenseId),
    Remove(DenseArrayOp),
}

#[derive(Debug)]
pub struct Slot2dId {
    pub row_id: u16,
    pub dense: DenseId,
}

#[derive(Debug)]
pub enum ColumnChange {
    Moved { new_offset: usize },
    Resized { new_size: usize },
}

impl<T: CommonBuffer + WriteBuffer> DenseMapped2d<T>
where
    T::Item: NoUninit,
{
    pub fn new(
        label: &str,
        ctx: &GraphicsCtx,
        data: impl Borrow<[T::Item]>,
        columns_size: impl IntoIterator<Item = u16>,
    ) -> Self {
        let data = data.borrow();
        let inner = T::new_vec(label, ctx, data);
        let mut offset_acc = 0;
        Self {
            inner,
            columns: columns_size
                .into_iter()
                .map(|c| ColumnMeta {
                    capacity: c as usize,
                    index_offset: {
                        let r = offset_acc;
                        offset_acc += c as usize;
                        r
                    },
                    changes: vec![],
                    ids: DenseIdAllocator::new_packed(c as u32),
                })
                .collect(),
            ttl_capacity: data.len(),

            #[cfg(debug_assertions)]
            label: label.to_string(),
        }
    }

    pub fn push(&mut self, column_id: u16, value: T::Item) -> Slot2dId {
        let column = &mut self.columns[column_id as usize];
        let id = column.ids.allocate();
        column.changes.push(ColumnOp::Insert(value, id));
        Slot2dId {
            row_id: column_id,
            dense: id,
        }
    }

    pub fn remove(&mut self, id: Slot2dId) {
        let column = &mut self.columns[id.row_id as usize];
        if let Some(array_op) = column.ids.free(id.dense) {
            column.changes.push(ColumnOp::Remove(array_op));
        }
    }

    pub fn apply_changes(&mut self, ctx: &GraphicsCtx) -> (bool, Vec<(u16, ColumnChange)>) {
        let mut changes = Vec::new();
        let new_capacities = self
            .columns
            .iter()
            .map(|c| {
                if c.ids.len() > c.capacity {
                    (c.capacity.max(1) * 2).max(c.ids.len())
                } else {
                    c.capacity
                }
            })
            .collect::<Box<_>>();
        let ttl_new_capacity = new_capacities.iter().sum::<usize>();

        if ttl_new_capacity > self.ttl_capacity {
            let new_buffer = T::new_empty_vec(&self.label, ctx, ttl_new_capacity);
            let mut encoder = ctx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Mapped2d Growable Buffer Copy Encoder"),
                });

            #[derive(Debug)]
            struct MoveBlock {
                old_offset: usize,
                new_offset: usize,
                size: usize,
            }

            let mut rest = MoveBlock {
                old_offset: 0,
                new_offset: 0,
                size: self.ttl_capacity,
            };
            let (mut old_block_size, mut new_block_size) = (0, 0); // Block size
            let mut prev_offset = 0;
            let mut move_needed = false;

            for (column_id, column) in self.columns.iter_mut().enumerate() {
                let old_cap = column.capacity;
                let new_cap = new_capacities[column_id];
                let grow = new_cap > old_cap;

                old_block_size += old_cap;
                new_block_size += new_cap;
                if grow {
                    encoder.copy_buffer_to_buffer(
                        self.inner.inner(),
                        rest.old_offset as u64 * T::ITEM_BYTE_SIZE,
                        new_buffer.inner(),
                        rest.new_offset as u64 * T::ITEM_BYTE_SIZE,
                        old_block_size as u64 * T::ITEM_BYTE_SIZE,
                    );
                    rest = MoveBlock {
                        old_offset: rest.old_offset + old_block_size,
                        new_offset: rest.new_offset + new_block_size,
                        size: rest.size - old_block_size,
                    };

                    new_block_size = 0;
                    old_block_size = 0;
                }

                // skip first columns before first update
                if move_needed {
                    column.index_offset = prev_offset;
                    changes.push((
                        column_id as u16,
                        ColumnChange::Moved {
                            new_offset: prev_offset,
                        },
                    ));
                } else if grow {
                    move_needed = true;
                }
                prev_offset += new_cap;
            }

            encoder.copy_buffer_to_buffer(
                self.inner.inner(),
                rest.old_offset as u64 * T::ITEM_BYTE_SIZE,
                new_buffer.inner(),
                rest.new_offset as u64 * T::ITEM_BYTE_SIZE,
                rest.size as u64 * T::ITEM_BYTE_SIZE,
            );

            ctx.queue.submit(Some(encoder.finish()));
            self.inner = new_buffer;
        }

        for (column_id, column) in self.columns.iter_mut().enumerate() {
            let mut size_diff = 0;
            for op in column.changes.drain(..) {
                match op {
                    ColumnOp::Insert(value, id) => {
                        if let Some(idx) = column.ids.get_index(id) {
                            size_diff += 1;
                            self.inner.write_at_index(
                                ctx,
                                &value,
                                column.index_offset as u32 + idx,
                            );
                        }
                    }
                    ColumnOp::Remove(op) => {
                        size_diff -= 1;
                        match op {
                            DenseArrayOp::RemoveLast {} => (),
                            DenseArrayOp::SwapRemove { index, last } => {
                                self.inner.swap_at_indices(ctx, index, last);
                            }
                        }
                    }
                }
                if size_diff != 0 {
                    changes.push((
                        column_id as u16,
                        ColumnChange::Resized {
                            new_size: column.ids.len() as usize,
                        },
                    ));
                }
            }
        }

        self.ttl_capacity = ttl_new_capacity;
        for (i, new_cap) in IntoIterator::into_iter(new_capacities).enumerate() {
            self.columns[i].capacity = new_cap;
        }

        (false, changes)
    }
}

impl<T: CommonBuffer> Deref for DenseMapped2d<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: CommonBuffer> DerefMut for DenseMapped2d<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
