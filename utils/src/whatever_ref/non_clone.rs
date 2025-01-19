use crate::whatever_ref::Storage;
use std::ops::Deref;

pub struct NonCloneWhateverRef<'a, T: ?Sized>(pub(super) NonCloneStorage<'a, T>);

pub(super) enum NonCloneStorage<'a, T: ?Sized> {
    Clone(Storage<'a, T>),
    NCDynDeref(Box<dyn Deref<Target = T> + 'a>),
}

impl<'a, T> From<Storage<'a, T>> for NonCloneStorage<'a, T> {
    fn from(value: Storage<'a, T>) -> Self {
        NonCloneStorage::Clone(value)
    }
}

impl<T: ?Sized> Deref for NonCloneWhateverRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match &self.0 {
            NonCloneStorage::Clone(r) => r,
            NonCloneStorage::NCDynDeref(r) => r,
        }
    }
}
