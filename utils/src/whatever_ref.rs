use crate::get_ref::GetRef;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

pub mod o_map;

pub use o_map::WhateverRefMap;

#[derive(Debug)]
pub struct WhateverRef<'a, T: ?Sized>(RegistryTypeRefStorage<'a, T>);

#[derive(Debug)]
pub struct RegistryRefMapLifetime<'a, T, R: 'a, F: Fn(&'a T) -> R>(WhateverRef<'a, T>, F);

#[derive(Debug)]
enum RegistryTypeRefStorage<'a, T: ?Sized> {
    Ref(&'a T),
    ArcRef(Arc<T>),
}

impl<'a, T> WhateverRef<'a, T> {
    pub fn map_ref<R: 'a, F: Fn(&'a T) -> R>(r: Self, f: F) -> RegistryRefMapLifetime<'a, T, R, F> {
        RegistryRefMapLifetime(r, f)
    }

    pub fn arc_from_owned(t: T) -> Self
    where
        T: 'static,
    {
        Self(RegistryTypeRefStorage::ArcRef(Arc::new(t)))
    }
}

impl<'a, T, R: 'a, F: Fn(&'a T) -> R> GetRef<'a, R> for RegistryRefMapLifetime<'a, T, R, F> {
    fn get_ref(&'a self) -> R {
        self.1(self.0.deref())
    }
}

impl<T: ?Sized> Deref for WhateverRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match &self.0 {
            RegistryTypeRefStorage::Ref(r) => r,
            RegistryTypeRefStorage::ArcRef(r) => r,
        }
    }
}

impl<'a, T: ?Sized> From<&'a T> for WhateverRef<'a, T> {
    fn from(r: &'a T) -> Self {
        Self(RegistryTypeRefStorage::Ref(r))
    }
}

impl<T: ?Sized> From<Arc<T>> for WhateverRef<'_, T> {
    fn from(r: Arc<T>) -> Self {
        Self(RegistryTypeRefStorage::ArcRef(r))
    }
}

impl<'a, T: ?Sized> Clone for RegistryTypeRefStorage<'a, T> {
    fn clone(&self) -> Self {
        match self {
            RegistryTypeRefStorage::Ref(r) => RegistryTypeRefStorage::Ref(r),
            RegistryTypeRefStorage::ArcRef(r) => RegistryTypeRefStorage::ArcRef(r.clone()),
        }
    }
}

impl<'a, T: ?Sized> Clone for WhateverRef<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
