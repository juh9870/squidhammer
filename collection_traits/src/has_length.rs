trait HasLength {
    fn len(&self) -> usize;
}

#[duplicate::duplicate_item(
    ty(T);
    [ Vec<T> ];
    [ [T] ];
)]
impl<T> HasLength for ty([T]) {
    fn len(&self) -> usize {
        self.len()
    }
}

impl<const N: usize, T> HasLength for [T; N] {
    fn len(&self) -> usize {
        N
    }
}
