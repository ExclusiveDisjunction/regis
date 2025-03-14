use super::{collect::CollectedMetrics, storage::LimitedQueue};
use common::error::PoisonError;
use common::lock::{ReadGuard, WriteGuard};
use std::collections::vec_deque::{Iter as VecIter, IterMut as VecIterMut, IntoIter as VecIntoIter};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use lazy_static::lazy_static;

pub const METRICS_HOLDING: usize = 50;

type Storage = LimitedQueue<CollectedMetrics>;

pub struct MetricProvider {
    inner: Arc<RwLock<LimitedQueue<CollectedMetrics>>>
}
impl Default for MetricProvider {
    fn default() -> Self {
        Self {
            inner: Arc::new(
                RwLock::new(
                    LimitedQueue::new(METRICS_HOLDING)
                )
            )
        }
    }
}
impl MetricProvider {
    

    pub fn push(&self, data: MetricProvider) {
        
    }

    pub fn access(&self) -> ReadGuard<'_, Storage> {
        self.inner
            .read()
            .map_err(|x| PoisonError::new(&x))
            .into()
    }
    pub fn access_mut(&self) -> WriteGuard<'_, Storage> {
        self.inner
            .write()
            .map_err(|x| PoisonError::new(&x))
            .into()
    }
}

lazy_static! {
    pub static ref METRICS: MetricProvider = MetricProvider::default();
}
