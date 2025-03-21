use std::{alloc::{alloc, dealloc, Layout}, marker::PhantomData, ops::{Index, IndexMut}, ptr::null};

pub struct LimitedQueue<T> where T: Sized{
    data: *mut T, 
    capacity: usize, 
    p: usize,
    count: usize
}
impl<T> LimitedQueue<T> where T: Sized {
    pub fn new(capacity: usize) -> Self {
        if capacity == 0 {
            panic!("The capacity cannot be zero.")
        }

        let layout = Layout::array::<T>(capacity).expect("Unable to get layout of object");
        let ptr: *mut T = unsafe { alloc(layout) as *mut T };

        Self {
            data: ptr,
            p: 0,
            capacity,
            count: 0
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn is_full(&self) -> bool {
        self.count == self.capacity
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

    pub fn get_at(&self, index: usize) -> Option<&T> {
        let front = self.front_index()?;

        if index >= self.count {
            return None;
        }

        let target_index = (index + front) % self.capacity;
        
        unsafe { self.data.add(target_index).as_ref() }
    }
    pub fn get_at_mut(&mut self, index: usize) -> Option<&mut T> {
        let front = self.front_index()?;

        if index >= self.count {
            return None;
        }

        let target_index = (index + front) % self.capacity;
        
        unsafe { self.data.add(target_index).as_mut() }
    }

    pub fn front(&self) -> Option<&T> {
        self.get_at(0)
    }
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.get_at_mut(0)
    }
    pub fn back(&self) -> Option<&T> {
        self.get_at(self.count - 1)
    }
    pub fn back_mut(&mut self) -> Option<&mut T> {
        self.get_at_mut(self.count - 1)
    }

    pub fn clear(&mut self) {
        self.p = 0;
        self.count = 0;
    }

    pub fn insert(&mut self, val: T) -> Option<T> where T: 'static {
        //We only return some value IF the capacity is not met.
        let will_return = self.is_full();

        // First set the value at the back index to be the new value.
        let curr_at_pos: &mut T = self.get_at_mut(self.p)?;
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
        self.iter().take(n).collect()
    }
    pub fn get_mut(&mut self, n: usize) -> Vec<&mut T> {
        self.iter_mut().take(n).collect()
    }
}
impl<T> Index<usize> for LimitedQueue<T> where T: Sized {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        self.get_at(index).expect("Index out of bounds")
    }
}
impl<T> IndexMut<usize> for LimitedQueue<T> where T: Sized {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_at_mut(index).expect("Index out of bounds")
    }
}
impl<T> Drop for LimitedQueue<T> where T: Sized {
    fn drop(&mut self) {
        if !self.data.is_null() {
            let layout = Layout::array::<T>(self.capacity).expect("Unable to get layout of object");
            unsafe {
                dealloc(self.data as *mut u8, layout);
                self.data = null::<T>() as *mut u8 as *mut T;
            }
        }
    }
}

pub struct LimitedQueueIter<'a, T> where T: Sized {
    data: &'a LimitedQueue<T>,
    p: Option<usize>,
    b: Option<usize>
}
impl<'a, T> Iterator for LimitedQueueIter<'a, T> where T: Sized {
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

        self.data.get_at(curr)
    }
}
impl<'a, T> LimitedQueueIter<'a, T> where T: Sized {
    pub fn new(parent: &'a LimitedQueue<T>) -> Self {
        Self {
            data: parent,
            p: parent.front_index(),
            b: parent.back_index()
        }
    }
}

pub struct LimitedQueueIterMut<'a, T> where T: Sized {
    data: *mut T,
    p: Option<usize>,
    b: Option<usize>,
    capacity: usize,
    len: usize,
    marker: std::marker::PhantomData<&'a mut T>
}
impl<'a, T> Iterator for LimitedQueueIterMut<'a, T> where T: Sized {
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
impl<T> LimitedQueueIterMut<'_, T> where T: Sized {
    pub fn new(source: &mut LimitedQueue<T>) -> Self {
        Self {
            data: source.data,
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