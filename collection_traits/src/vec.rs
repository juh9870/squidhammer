use crate::{AsSlice, Resizable};

impl<T> AsSlice for Vec<T> {
    type Item = T;

    fn as_slice(&self) -> &[T] {
        self.as_slice()
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> Resizable for Vec<T> {
    type Item = T;

    fn resize_with(&mut self, new_len: usize, f: impl FnMut() -> Self::Item) {
        self.resize_with(new_len, f)
    }

    fn push(&mut self, item: Self::Item) {
        self.push(item)
    }

    fn pop(&mut self) -> Option<Self::Item> {
        self.pop()
    }

    fn insert(&mut self, index: usize, item: Self::Item) {
        self.insert(index, item)
    }

    fn remove(&mut self, index: usize) -> Self::Item {
        self.remove(index)
    }

    fn swap_remove(&mut self, index: usize) -> Self::Item {
        self.swap_remove(index)
    }
}
