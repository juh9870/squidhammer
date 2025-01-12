use crate::HasLength;

impl<T> HasLength for [T] {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}
