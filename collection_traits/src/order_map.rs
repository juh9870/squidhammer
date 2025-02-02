use crate::{HasLength, Iterable};

#[allow(clippy::disallowed_types)]
impl<K, V, H> Iterable for ordermap::OrderMap<K, V, H> {
    type Item<'a> = (&'a K, &'a V) where Self: 'a;

    fn iter<'a>(&'a self) -> impl Iterator<Item = Self::Item<'a>> {
        ordermap::OrderMap::iter(self)
    }
}

#[allow(clippy::disallowed_types)]
impl<K, V, H> HasLength for ordermap::OrderMap<K, V, H> {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

#[allow(clippy::disallowed_types)]
impl<V, H> Iterable for ordermap::OrderSet<V, H> {
    type Item<'a> = &'a V where Self: 'a;

    fn iter<'a>(&'a self) -> impl Iterator<Item = Self::Item<'a>> {
        ordermap::OrderSet::iter(self)
    }
}

#[allow(clippy::disallowed_types)]
impl<V, H> HasLength for ordermap::OrderSet<V, H> {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}
