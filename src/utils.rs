use std::collections::VecDeque;

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
