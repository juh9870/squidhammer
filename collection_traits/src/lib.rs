#![forbid(clippy::unconditional_recursion)]

pub mod slice;
pub mod vec;

#[cfg(feature = "smallvec")]
pub mod smallvec;

pub trait HasLength {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

pub trait Resizable {
    type Item;
    fn resize_with(&mut self, new_len: usize, f: impl FnMut() -> Self::Item);
    fn push(&mut self, item: Self::Item);
    fn pop(&mut self) -> Option<Self::Item>;
    fn insert(&mut self, index: usize, item: Self::Item);
    fn remove(&mut self, index: usize) -> Self::Item;
    fn swap_remove(&mut self, index: usize) -> Self::Item;
}
