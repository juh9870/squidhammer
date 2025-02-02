use crate::{HasLength, Iterable, Resizable};
use arrayvec::ArrayVec;

impl<const N: usize, T> Resizable for ArrayVec<T, N> {
    type Item = T;

    fn resize_with(&mut self, new_len: usize, mut f: impl FnMut() -> Self::Item) {
        if self.len() > new_len {
            self.truncate(new_len);
        } else if self.len() < new_len {
            for _ in 0..new_len - self.len() {
                self.push(f());
            }
        }
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

impl<const N: usize, T> HasLength for ArrayVec<T, N> {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<const N: usize, T> Iterable for ArrayVec<T, N> {
    type Item<'a> = &'a T where Self: 'a;

    fn iter<'a>(&'a self) -> impl Iterator<Item = Self::Item<'a>> {
        self.as_slice().iter()
    }
}
