use crate::{HasLength, Iterable};

impl<T> HasLength for [T] {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T> Iterable for [T] {
    type Item<'a> = &'a T where Self: 'a;

    #[expect(clippy::needless_lifetimes)]
    fn iter<'a>(&'a self) -> impl Iterator<Item = Self::Item<'a>> {
        <[T]>::iter(self)
    }
}
