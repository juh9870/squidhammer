#![forbid(unconditional_recursion)]
#![deny(clippy::disallowed_types)]

pub mod hash_map;
pub mod slice;
pub mod vec;

#[cfg(feature = "smallvec")]
pub mod smallvec;

#[cfg(feature = "arrayvec")]
pub mod arrayvec;

#[cfg(feature = "ordermap")]
pub mod order_map;

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

pub trait Iterable {
    type Item<'a>
    where
        Self: 'a;

    #[expect(clippy::needless_lifetimes)]
    fn iter<'a>(&'a self) -> impl Iterator<Item = Self::Item<'a>>;
}
