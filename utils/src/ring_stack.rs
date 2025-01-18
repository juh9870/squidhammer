use std::collections::VecDeque;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Default)]
pub struct RingStack<T> {
    stack: VecDeque<T>,
    max_size: usize,
}

impl<T> RingStack<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            stack: VecDeque::new(),
            max_size,
        }
    }

    pub fn push(&mut self, item: T) {
        self.ensure_free_spot();
        self.stack.push_back(item);
    }

    pub fn insert(&mut self, index: usize, item: T) {
        self.ensure_free_spot();
        self.stack.insert(index, item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop_back()
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        self.stack.remove(index)
    }

    pub fn swap_remove(&mut self, index: usize) -> Option<T> {
        self.stack.swap_remove_back(index)
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.stack.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::vec_deque::IterMut<T> {
        self.stack.iter_mut()
    }

    pub fn into_iter(self) -> std::collections::vec_deque::IntoIter<T> {
        self.stack.into_iter()
    }

    pub fn drain(&mut self) -> std::collections::vec_deque::Drain<T> {
        self.stack.drain(..)
    }
}

impl<T> RingStack<T> {
    fn ensure_free_spot(&mut self) {
        if self.stack.len() == self.max_size {
            self.stack.pop_front();
        }
    }
}

impl<T> collection_traits::HasLength for RingStack<T> {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T> collection_traits::Resizable for RingStack<T> {
    type Item = T;

    fn resize_with(&mut self, new_len: usize, f: impl FnMut() -> Self::Item) {
        if new_len != self.max_size {
            self.max_size = new_len;
        }

        self.stack.resize_with(new_len, f);
    }

    fn push(&mut self, item: Self::Item) {
        self.push(item);
    }

    fn pop(&mut self) -> Option<Self::Item> {
        self.pop()
    }

    fn insert(&mut self, index: usize, item: Self::Item) {
        self.stack.insert(index, item);
    }

    fn remove(&mut self, index: usize) -> Self::Item {
        self.remove(index).unwrap_or_else(|| {
            panic!(
                "Index out of bounds: the length is {} but the index is {}",
                self.stack.len(),
                index
            )
        })
    }

    fn swap_remove(&mut self, index: usize) -> Self::Item {
        self.swap_remove(index).unwrap_or_else(|| {
            panic!(
                "Index out of bounds: the length is {} but the index is {}",
                self.stack.len(),
                index
            )
        })
    }
}

#[cfg(test)]
mod test {
    use crate::ring_stack::RingStack;
    use itertools::Itertools;

    #[test]
    fn test() {
        let mut q = RingStack::new(2);

        q.push(1);
        q.push(2);
        q.push(3);

        assert_eq!(q.iter().copied().collect_vec(), vec![2, 3]);

        assert_eq!(q.pop(), Some(3));
        assert_eq!(q.pop(), Some(2));
    }
}
