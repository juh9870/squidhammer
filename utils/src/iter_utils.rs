use itertools::Itertools;
use thiserror::Error;

#[derive(Debug, Copy, Clone, Error)]
#[error("{} items were requested, but only {} were found", .wants, .found)]
pub struct NotEnoughError {
    pub wants: usize,
    pub found: usize,
}

impl NotEnoughError {
    pub fn new(wants: usize, found: usize) -> Self {
        Self { wants, found }
    }
}

pub trait UtilsNextNIterExt<T>: Iterator<Item = T> {
    fn next_n<const N: usize>(&mut self) -> Result<[T; N], NotEnoughError>;
}

impl<T, I: Iterator<Item = T>> UtilsNextNIterExt<T> for I {
    fn next_n<const N: usize>(&mut self) -> Result<[T; N], NotEnoughError> {
        let arr: Vec<T> = [(); N]
            .into_iter()
            .enumerate()
            .map(|(i, _)| self.next().ok_or_else(|| NotEnoughError::new(N, i)))
            .try_collect()?;
        Ok(arr
            .try_into()
            .map_err(|_| unreachable!("Length is checked before"))
            .unwrap())
    }
}
