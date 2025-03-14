use super::{collect::CollectedMetrics, storage::LimitedQueue};
use common::error::PoisonError;
use common::lock::{ReadGuard, WriteGuard};
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
    pub fn reset(&self) {
        {
            let mut guard = match self.inner.write() {
                Ok(g) => g,
                Err(e) => e.into_inner()
            };
    
            *guard = LimitedQueue::new(METRICS_HOLDING);
        }
        
        self.inner.clear_poison();
    }
    
    pub fn push(&self, data: CollectedMetrics) {
        let mut guard = self.access_mut_or_reset();
        guard.insert(data);
    }
    pub fn view(&self, n: usize) -> Vec<CollectedMetrics> {
        let guard = self.access_or_reset(false);

        guard.get(n).into_iter().map(|x| x.clone()).collect()
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

    fn access_or_reset(&self, prev: bool) -> RwLockReadGuard<'_, Storage> {
        if let Ok(v) = self.inner.read() {
            v
        }
        else {
            if prev {
                panic!("Unable to get value out of lock 2 times in a row.");
            }

            {
                let mut write = match self.inner.write() {
                    Ok(w) => w,
                    Err(e) => e.into_inner()
                };

                *write = LimitedQueue::new(METRICS_HOLDING);
            }

            self.access_or_reset(true)            
        }
    }
    fn access_mut_or_reset(&self) -> RwLockWriteGuard<'_, Storage> {
        match self.inner.write() {
            Ok(v) => v,
            Err(e) => {
                let mut guard = e.into_inner();
                *guard = LimitedQueue::new(METRICS_HOLDING);

                guard
            }
        }
    }
}

lazy_static! {
    pub static ref METRICS: MetricProvider = MetricProvider::default();
}
