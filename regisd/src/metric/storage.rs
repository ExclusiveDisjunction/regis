
pub struct QueueEntry<T> {
    value: T,
    next: Option<Box<QueueEntry<T>>>,
    last: Option<Box<QueueEntry<T>>>
}

pub struct LimitedQueue<T> where {
    start: Option<QueueEntry<T>>,
    stop: Option<QueueEntry<T>>,
    size: usize,
    cutoff: usize
}
impl<T> LimitedQueue<T> {
    pub fn new(cutoff: usize) -> Self {
        Self {
            start: None,
            stop: None,
            size: 0,
            cutoff
        }
    }
}