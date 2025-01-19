use crate::whatever_ref::WhateverRef;
use std::ops::Deref;

pub struct WhateverRefMap<'a, T: ?Sized, R: ?Sized + 'a>(SelfRefMap<'a, T, R>);

impl<'a, T> WhateverRef<'a, T> {
    pub fn map<R: ?Sized>(
        data: WhateverRef<'a, T>,
        map: impl for<'this> Fn(&'this WhateverRef<'a, T>) -> &'this R,
    ) -> WhateverRefMap<'a, T, R> {
        WhateverRefMap(SelfRefMap::new(data, map))
    }

    pub fn try_map<R: ?Sized, Err>(
        data: WhateverRef<'a, T>,
        map: impl for<'this> Fn(&'this WhateverRef<'a, T>) -> Result<&'this R, Err>,
    ) -> Result<WhateverRefMap<'a, T, R>, Err> {
        Ok(WhateverRefMap(SelfRefMap::try_new(data, map)?))
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
