use smallvec::SmallVec;

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

macro_rules! apply_fn {
    ($self:ident, $vec:ident) => {
        match $self {
            VecOperation::ShiftRemove(idx) => {
                $vec.remove(idx);
            }
            VecOperation::SwapRemove(idx) => {
                $vec.swap_remove(idx);
            }
            VecOperation::Push(data) => $vec.push(data),
            VecOperation::Insert(idx, data) => $vec.insert(idx, data),
            VecOperation::Replace(idx, data) => $vec[idx] = data,
            VecOperation::Move(from, to) => {
                if to == from {
                    return;
                }
                let data = $vec.remove(from);
                $vec.insert(to, data);
            }
            VecOperation::Swap(from, to) => {
                $vec.swap(from, to);
            }
        }
    };
}

impl<T> VecOperation<T> {
    pub fn apply_vec(self, vec: &mut Vec<T>) {
        apply_fn!(self, vec);
    }

    pub fn apply_smallvec<const N: usize>(self, vec: &mut SmallVec<[T; N]>) {
        apply_fn!(self, vec);
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
