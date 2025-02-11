use std::{collections::VecDeque, ops::Range};

use egui::ahash::HashMap;

#[derive(Default)]
pub struct IdAllocator<T = u32> {
    free_ids: VecDeque<T>,
    len: T,
}

impl<T: Default + std::ops::AddAssign + From<u8> + Copy> IdAllocator<T> {
    pub fn new_packed(len: T) -> Self {
        Self {
            len,
            ..Default::default()
        }
    }

    pub fn allocate(&mut self) -> T {
        if let Some(id) = self.free_ids.pop_front() {
            id
        } else {
            let id = self.len;
            self.len += 1.into();
            id
        }
    }

    pub fn free(&mut self, id: T) {
        self.free_ids.push_back(id);
    }

    pub fn len(&self) -> T {
        self.len
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct DenseId(u32);

#[derive(Default)]
pub struct DenseIdAllocator {
    to_index: HashMap<DenseId, usize>,
    from_index: Vec<DenseId>,
    next_dense: u32,
}

pub enum ArrayOp {
    SwapRemove(u32),
    RemoveLast,
}

impl DenseIdAllocator {
    pub fn new_packed(len: u32) -> Self {
        let from_index: Vec<_> = (0..len).map(|i| DenseId(i)).collect();
        let mut i = 0;
        let to_index = from_index
            .iter()
            .map(|h| {
                (*h, {
                    let r = i;
                    i += 1;
                    r
                })
            })
            .collect();
        Self {
            to_index,
            from_index,
            next_dense: len,
        }
    }

    /// This should be followed by a push into the dense array
    pub fn allocate(&mut self) -> DenseId {
        let len = self.len();
        let handle = DenseId(self.next_dense);
        self.next_dense += 1;

        self.to_index.insert(handle, self.len());
        self.from_index.push(handle);

        handle
    }

    pub fn free(&mut self, handle: DenseId) -> Option<ArrayOp> {
        let index = *self.to_index.get(&handle)?;

        let last_index = self.len() - 1;
        self.from_index.swap(index as usize, last_index as usize);

        let removed_handle = self.from_index.pop().unwrap();
        self.to_index.remove(&handle);

        Some(if index != last_index {
            self.to_index.insert(removed_handle, index);
            ArrayOp::SwapRemove(index as u32)
        } else {
            ArrayOp::RemoveLast
        })
    }

    pub fn get_index(&self, id: DenseId) -> Option<u32> {
        self.to_index.get(&id).map(|i| *i as u32)
    }

    pub fn len(&self) -> usize {
        assert!(self.to_index.len() == self.from_index.len());
        self.from_index.len()
    }
}
