use crate::{AsSlice, HasLength};

impl<T> HasLength for [T] {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T> AsSlice for [T] {
    type Item = T;

    fn as_slice(&self) -> &[T] {
        self
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }
}

impl<T: AsSlice> HasLength for T {
    fn len(&self) -> usize {
        self.as_slice().len()
    }

    fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
}
