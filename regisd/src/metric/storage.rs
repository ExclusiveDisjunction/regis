use std::collections::VecDeque;
use std::collections::vec_deque::{Iter as VecIter, IntoIter as VecIntoIter, IterMut as VecIterMut};

pub struct LimitedQueue<T> where {
    data: VecDeque<T>,
    cutoff: usize
}
impl<T> LimitedQueue<T> {
    pub fn new(cutoff: usize) -> Self {
        Self {
            data: VecDeque::new(),
            cutoff
        }
    }

    pub fn previous_inserted(&self) -> Option<&T> {
        self.data.front()
    }
    pub fn previous_inserted_mut(&mut self) -> Option<&mut T> {
        self.data.front_mut()
    }
    pub fn first_inserted(&self) -> Option<&T> {
        self.data.back()
    }
    pub fn first_inserted_mut(&mut self) -> Option<&mut T> {
        self.data.back_mut()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn insert(&mut self, val: T) -> Option<T> where T: 'static {
        if self.cutoff == 0 {
            return None;
        }

        self.data.push_front(val);

        if self.data.len() > self.cutoff {
            self.data.pop_back()
        }
        else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    pub fn cutoff(&self) -> usize {
        self.cutoff
    }

    pub fn iter(&self) -> VecIter<'_, T> {
        self.data.iter()
    }
    pub fn iter_mut(&mut self) -> VecIterMut<'_, T> {
        self.data.iter_mut()
    }
    pub fn into_iter(self) -> VecIntoIter<T> {
        self.data.into_iter()
    }

    pub fn get(&self, n: usize) -> Vec<&T> {
        self.data.iter().take(n).collect()
    }
    pub fn get_mut(&mut self, n: usize) -> Vec<&mut T> {
        self.data.iter_mut().take(n).collect()
    }
}
impl<T> From<LimitedQueue<T>> for Vec<T> {
    fn from(value: LimitedQueue<T>) -> Self {
        value.into_iter().collect()
    }
}
impl<T> From<LimitedQueue<T>> for VecDeque<T> {
    fn from(value: LimitedQueue<T>) -> Self {
        value.data
    }
}

#[test]
fn test_limited_queue() {
    let mut queue: LimitedQueue<i32> = LimitedQueue::new(3);

    assert_eq!(queue.insert(1), None);
    assert_eq!(queue.insert(2), None);
    assert_eq!(queue.insert(3), None);
    assert_eq!(queue.insert(4), Some(1));
    assert_eq!(queue.insert(5), Some(2));

    let as_vec: Vec<i32> = queue.into();
    assert_eq!(as_vec, vec![5, 4, 3]);
}