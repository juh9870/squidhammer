use crate::whatever_ref::non_clone::{NonCloneStorage, NonCloneWhateverRef};
use crate::whatever_ref::{Storage, WhateverRef};
use std::fmt::Debug;
use std::ops::Deref;

pub struct WhateverRefMap<'a, T: ?Sized, R: ?Sized + 'a>(SelfRefMap<'a, T, R>);
pub struct WhateverRefCallMap<'a, T: ?Sized, R: ?Sized + 'a, F: Fn(&T) -> &R + Clone + 'a>(
    SelfRefCallMap<'a, T, R, F>,
);

impl<'a, T> WhateverRef<'a, T> {
    /// Maps the inner reference to another reference type.
    ///
    /// The provided closure is called with the inner reference during
    /// initialization and whenever the reference is cloned.
    pub fn call_map<R: ?Sized, F: Fn(&T) -> &R + Clone + 'a>(
        data: WhateverRef<'a, T>,
        map: F,
    ) -> WhateverRefCallMap<'a, T, R, F> {
        WhateverRefCallMap(SelfRefCallMap::new(data, map.clone(), |x| map(x)))
    }

    /// Maps the inner reference to another reference type.
    ///
    /// The provided closure is called with the inner reference during
    /// initialization and whenever the reference is cloned.
    ///
    /// If the closure returns an error, the error is returned.
    ///
    /// Closure is expected to be deterministic. It should either fail every
    /// time or succeed every time when called with the same arguments.
    pub fn try_call_map<R: ?Sized, E: Debug, F: Fn(&T) -> Result<&R, E> + Clone + 'a>(
        data: WhateverRef<'a, T>,
        map: F,
    ) -> Result<WhateverRefCallMap<'a, T, R, impl Fn(&T) -> &R + Clone + 'a>, E> {
        let cloned = map.clone();
        Ok(WhateverRefCallMap(SelfRefCallMap::try_new(
            data,
            move |x| cloned(x).expect("try_call_map closure should be deterministic"),
            |x| map(x),
        )?))
    }

    /// Maps the inner reference to another reference type.
    ///
    /// The provided closure is called with the inner reference during
    /// initialization.
    ///
    /// Resulting reference is non-cloneable. Use [`WhateverRef::call_map`] if you
    /// need a cloneable reference.
    pub fn map<R: ?Sized>(
        data: WhateverRef<'a, T>,
        map: impl for<'this> FnOnce(&'this T) -> &'this R,
    ) -> WhateverRefMap<'a, T, R> {
        WhateverRefMap(SelfRefMap::new(data, |x| map(x)))
    }

    /// Maps the inner reference to another reference type.
    ///
    /// The provided closure is called with the inner reference during
    /// initialization. If the closure returns an error, the error is returned.
    ///
    /// Resulting reference is non-cloneable.  Use [`WhateverRef::try_call_map`]
    /// if you need a cloneable reference.
    pub fn try_map<R: ?Sized, Err>(
        data: WhateverRef<'a, T>,
        map: impl for<'this> FnOnce(&'this T) -> Result<&'this R, Err>,
    ) -> Result<WhateverRefMap<'a, T, R>, Err> {
        Ok(WhateverRefMap(SelfRefMap::try_new(data, |x| map(x))?))
    }
}
impl<'a, T: ?Sized, R: ?Sized + 'a> WhateverRefMap<'a, T, R> {
    pub fn into_dyn_ref(self) -> NonCloneWhateverRef<'a, R> {
        NonCloneWhateverRef(NonCloneStorage::NCDynDeref(Box::new(self)))
    }
}
impl<'a, T: ?Sized, R: ?Sized + 'a, F: Fn(&T) -> &R + Clone + 'a> WhateverRefCallMap<'a, T, R, F> {
    pub fn into_dyn_ref(self) -> WhateverRef<'a, R> {
        WhateverRef(Storage::DynDeref(Box::new(self)))
    }
}

#[ouroboros::self_referencing]
pub struct SelfRefMap<'a, T: ?Sized, R: ?Sized + 'a> {
    data: WhateverRef<'a, T>,
    #[borrows(data)]
    mapped: &'this R,
}

impl<'a, T: ?Sized, R: ?Sized + 'a> Deref for WhateverRefMap<'a, T, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.0.borrow_mapped()
    }
}

#[ouroboros::self_referencing]
pub struct SelfRefCallMap<'a, T: ?Sized, R: ?Sized + 'a, F: Fn(&T) -> &R + Clone + 'a> {
    data: WhateverRef<'a, T>,
    call: F,
    #[borrows(data)]
    mapped: &'this R,
}

impl<'a, T: ?Sized, R: ?Sized + 'a, F: Fn(&T) -> &R + Clone + 'a> Clone
    for SelfRefCallMap<'a, T, R, F>
{
    fn clone(&self) -> Self {
        let data = self.borrow_data().clone();
        let call = self.borrow_call().clone();
        Self::new(data, call.clone(), |x| call(x))
    }
}

impl<'a, T: ?Sized, R: ?Sized + 'a, F: Fn(&T) -> &R + Clone + 'a> Clone
    for WhateverRefCallMap<'a, T, R, F>
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, T: ?Sized, R: ?Sized + 'a, F: Fn(&T) -> &R + Clone + 'a> Deref
    for WhateverRefCallMap<'a, T, R, F>
{
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.0.borrow_mapped()
    }
}
