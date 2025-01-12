use crate::AsSlice;

impl<const N: usize, T> AsSlice for [T; N] {
    type Item = T;

    fn as_slice(&self) -> &[T] {
        self.as_slice()
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}
