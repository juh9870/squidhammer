use collection_traits::Resizable;

#[derive(Debug, Copy, Clone)]
pub enum VecOperation<T> {
    ShiftRemove(usize),
    SwapRemove(usize),
    Push(T),
    Insert(usize, T),
    Replace(usize, T),
    Move(usize, usize),
    Swap(usize, usize),
}

impl<T> VecOperation<T> {
    pub fn apply<Col: AsMut<[T]> + Resizable<Item = T>>(self, vec: &mut Col) {
        match self {
            VecOperation::ShiftRemove(idx) => {
                vec.remove(idx);
            }
            VecOperation::SwapRemove(idx) => {
                vec.swap_remove(idx);
            }
            VecOperation::Push(data) => vec.push(data),
            VecOperation::Insert(idx, data) => vec.insert(idx, data),
            VecOperation::Replace(idx, data) => vec.as_mut()[idx] = data,
            VecOperation::Move(from, to) => {
                if to == from {
                    return;
                }
                let data = vec.remove(from);
                vec.insert(to, data);
            }
            VecOperation::Swap(from, to) => {
                vec.as_mut().swap(from, to);
            }
        }
    }
}

/// Create a `SmallVec` with a specified size and elements.
#[macro_export]
macro_rules! smallvec_n {
    ($size:literal; $($x:expr),*$(,)*) => {
        {
            let sv: smallvec::SmallVec<[_; $size]> = smallvec::smallvec![$($x),*];
            sv
        }
    }
}
