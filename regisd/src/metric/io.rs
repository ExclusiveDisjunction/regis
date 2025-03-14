use super::{collect::CollectedMetrics, storage::LimitedQueue};
use std::ops::Deref;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;

pub const METRICS_HOLDING: usize = 50;

pub struct MetricsCollection {
    metrics: LimitedQueue<CollectedMetrics>,
    events: HashMap<u32, Arc<dyn Fn(CollectedMetrics) -> () + Send + Sync + 'static>>,
    curr_id: u32
}
impl Default for MetricsCollection {
    fn default() -> Self {
        Self {
            metrics: LimitedQueue::new(METRICS_HOLDING),
            events: HashMap::new(),
            curr_id: 0
        }
    }
}
impl MetricsCollection {
    fn get_next_id(&mut self) -> u32 {
        let result = self.curr_id;
        self.curr_id += 1;

        result
    }

    pub fn push(&mut self, metric: CollectedMetrics) {
        self.metrics.insert(metric.clone());

        self.notify(metric)
    }
    pub fn notify(&self, metric: CollectedMetrics) {
        for (_, event) in &self.events {
            let func = event.deref();
            func(metric.clone())
        }
    }

    pub fn subscribe<F>(&mut self, func: F) -> u32 
    where F: Fn(CollectedMetrics) -> () + Send + Sync + 'static {
        let id = self.get_next_id();
        self.events.insert(id, Arc::new(func));

        id
    }
    pub fn unsubscribe(&mut self, id: u32) -> bool {
        self.events.remove(&id).is_some()
    }
}

pub struct MetricProvider {
    inner: Arc<RwLock<MetricsCollection>>
}
impl Default for MetricProvider {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MetricsCollection::default()))
        }
    }
}

lazy_static! {
    pub static ref METRICS: MetricProvider = MetricProvider::default();
}
