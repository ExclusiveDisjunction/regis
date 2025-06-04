use std::marker::PhantomData;

pub struct LimitedQueue<T> where T: Default + Clone {
    data: Vec<T>,
    p: usize,
    count: usize
}
impl<T> LimitedQueue<T> where T: Default + Clone {
    pub fn new(capacity: usize) -> Self {
        if capacity == 0 {
            panic!("The capacity cannot be zero.")
        }
        Self {
            data: vec![T::default(); capacity],
            p: 0,
            count: 0
        }
    }

    pub fn capacity(&self) -> usize {
        self.data.len()
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn is_full(&self) -> bool {
        self.count >= self.capacity()
    }

    pub fn front_index(&self) -> Option<usize> {
        if self.is_empty() {
            None
        }
        else if self.p == 0 || !self.is_full() {
            Some(0)
        }
        else {
            Some(self.p)
        }
    }
    pub fn back_index(&self) -> Option<usize> {
        if self.is_empty() {
            None
        }
        else if self.p != 0 {
            Some(self.p - 1)
        }
        else {
            Some(self.capacity() - 1)
        }
    }

    pub fn front(&self) -> Option<&T> {
        self.data.get(self.front_index()?)
    }
    pub fn front_mut(&mut self) -> Option<&mut T> {
        let index = self.front_index()?;
        self.data.get_mut(index)
    }
    pub fn back(&self) -> Option<&T> {
        self.data.get(self.back_index()?)
    }
    pub fn back_mut(&mut self) -> Option<&mut T> {
        let index = self.back_index()?;
        self.data.get_mut(index)
    }

    pub fn clear(&mut self) {
        self.p = 0;
        self.count = 0;
    }

    pub fn insert(&mut self, val: T) -> Option<T> where T: 'static {
        //We only return some value IF the capacity is not met.
        let will_return = self.is_full();

        // First set the value at the back index to be the new value.
        let curr_at_pos: &mut T = self.data.get_mut(self.p)?;
        let result = std::mem::replace(curr_at_pos, val);

        // Update the back
        self.p = (self.p + 1) % self.capacity();
        if !self.is_full() {
            self.count += 1;
        }

        if will_return {
            Some(result)
        }
        else {
            None
        }
    }

    pub fn iter(&self) -> LimitedQueueIter<'_, T> {
        LimitedQueueIter::new(self)
    }
    pub fn iter_mut(&mut self) -> LimitedQueueIterMut<'_, T> {
        LimitedQueueIterMut::new(self)
    }
    pub fn get(&self, n: usize) -> Vec<&T> {
        self.data.iter().take(n).collect()
    }
    pub fn get_mut(&mut self, n: usize) -> Vec<&mut T> {
        self.data.iter_mut().take(n).collect()
    }
}

pub struct LimitedQueueIter<'a, T> where T: Default + Clone {
    data: &'a LimitedQueue<T>,
    p: Option<usize>,
    b: Option<usize>
}
impl<'a, T> Iterator for LimitedQueueIter<'a, T> where T: Default + Clone {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.p?;
        let last = self.b?;
        let next: usize = (curr + 1) % self.data.capacity();

        if curr == last {
            self.p = None
        }
        else {
            self.p = Some(next)
        }

        self.data.data.get(curr)
    }
}
impl<'a, T> LimitedQueueIter<'a, T> where T: Default + Clone {
    pub fn new(parent: &'a LimitedQueue<T>) -> Self {
        Self {
            data: parent,
            p: parent.front_index(),
            b: parent.back_index()
        }
    }
}

pub struct LimitedQueueIterMut<'a, T> where T: Default + Clone {
    data: *mut T,
    p: Option<usize>,
    b: Option<usize>,
    capacity: usize,
    len: usize,
    marker: std::marker::PhantomData<&'a mut T>
}
impl<'a, T> Iterator for LimitedQueueIterMut<'a, T> where T: Default + Clone {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.p?;
        let last = self.b?;
        let next = (curr + 1) % self.capacity;

        if curr == last {
            self.p = None;
        }
        else {
            self.p = Some(next);
        }

        if curr >= self.len {
            None
        }
        else {
            unsafe {
                let item = &mut *self.data.add(curr);
                Some(item)
            }
        }
    }
}
impl<T> LimitedQueueIterMut<'_, T> where T: Default + Clone {
    pub fn new(source: &mut LimitedQueue<T>) -> Self {
        Self {
            data: source.data.as_mut_ptr(),
            p: source.front_index(),
            b: source.back_index(),
            capacity: source.capacity(),
            len: source.len(),
            marker: PhantomData
        }
    }
}

#[test]
fn test_limited_queue() {
    let mut queue: LimitedQueue<i32> = LimitedQueue::new(3);

    assert_eq!(queue.insert(1), None);
    assert_eq!(queue.insert(2), None);
    assert_eq!(queue.insert(3), None);

    assert_eq!(queue.front(), Some(&1));
    assert_eq!(queue.back(), Some(&3));

    assert_eq!(queue.insert(4), Some(1));
    assert_eq!(queue.insert(5), Some(2));

    assert_eq!(queue.back(), Some(&5));
    assert_eq!(queue.front(), Some(&3));

    let collected: Vec<i32> = queue.iter().cloned().collect();
    assert_eq!(collected, vec![3, 4, 5]);
}