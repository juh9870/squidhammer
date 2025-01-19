use std::ops::Deref;

pub trait GetRef<'a, T: ?Sized + 'a> {
    fn get_ref(&'a self) -> T;
}

impl<'a, T: Deref<Target = R>, R: ?Sized> GetRef<'a, &'a R> for T {
    fn get_ref(&'a self) -> &'a R {
        self
    }
}
