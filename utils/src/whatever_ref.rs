use dyn_clone::DynClone;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

pub mod non_clone;
pub mod o_map;

pub use o_map::WhateverRefMap;

#[derive(Debug)]
pub struct WhateverRef<'a, T: ?Sized>(Storage<'a, T>);

enum Storage<'a, T: ?Sized> {
    Ref(&'a T),
    Arc(Arc<T>),
    DynDeref(Box<dyn CloneableDeref<Target = T> + 'a>),
}

impl<'a, T> WhateverRef<'a, T> {
    pub fn arc_from_owned(t: T) -> Self
    where
        T: 'static,
    {
        Self(Storage::Arc(Arc::new(t)))
    }

    pub fn from_ref(r: &'a T) -> Self {
        Self(Storage::Ref(r))
    }

    pub fn from_arc(arc: Arc<T>) -> Self {
        Self(Storage::Arc(arc))
    }

    pub fn from_dyn_deref(d: Box<dyn CloneableDeref<Target = T> + 'a>) -> Self {
        Self(Storage::DynDeref(d))
    }
}

impl<'a, T: ?Sized> Clone for WhateverRef<'a, T> {
    fn clone(&self) -> Self {
        match &self.0 {
            Storage::Ref(r) => Self(Storage::Ref(r)),
            Storage::Arc(r) => Self(Storage::Arc(r.clone())),
            Storage::DynDeref(r) => Self(Storage::DynDeref(dyn_clone::clone_box(&**r))),
        }
    }
}

impl<T: ?Sized> Deref for WhateverRef<'_, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: ?Sized> Deref for Storage<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match &self {
            Self::Ref(r) => r,
            Self::Arc(r) => r,
            Self::DynDeref(r) => r,
        }
    }
}

impl<'a, T: ?Sized + Debug> Debug for Storage<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ref(r) => write!(f, "Ref({:?})", r),
            Self::Arc(r) => write!(f, "Arc({:?})", r),
            Self::DynDeref(_) => write!(f, "DynDeref(..)"),
        }
    }
}

pub trait CloneableDeref: Deref + DynClone {}

impl<T: Deref + DynClone + ?Sized> CloneableDeref for T {}
