use common::metric::CollectedMetrics;

use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;

use exdisj::log_error;
use exdisj::storage::LimitedQueue;
use exdisj::lock::{RwProvider, RwProviderAccess, ProtectedAccess};

use tokio::sync::broadcast::{Sender, Receiver, channel};

pub const METRICS_HOLDING: usize = 50;

type Storage = LimitedQueue<CollectedMetrics>;

// Thing to improve: Include a mechanism that will eventually hold the discarded items from memory. After that buffer fills up, write it to a file, with a specified number of files kept. This way there is some more historical data, but not stored in memory all of the time.

pub struct MetricsEvents {
    sender: Arc<Sender<CollectedMetrics>>
}
impl Default for MetricsEvents {
    fn default() -> Self {
        let (sender, _) = channel(10);

        Self {
            sender: Arc::new(sender)
        }
    }
}
impl MetricsEvents {
    pub fn subscribe(&self) -> Receiver<CollectedMetrics> {
        self.sender.subscribe()
    }

    pub fn notify(&self, value: CollectedMetrics) -> bool {
        if let Err(e) = self.sender.send(value) {
            log_error!("Unable to send out subscription notification '{e}'");
            false
        }
        else {
            true
        }
    }
}

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
impl RwProvider for MetricProvider {
    type Data = Storage;

    fn access_raw(&self) -> ProtectedAccess<'_, Arc<RwLock<Self::Data>>> {
        ProtectedAccess::new(&self.inner)
    }
}
impl RwProviderAccess for MetricProvider {}
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
    
    pub fn push(&self, data: CollectedMetrics) -> bool {
        self.access_mut()
            .access()
            .map(|x| x.insert(data))
            .is_some()
    }
    pub fn view(&self, n: usize) -> Option<Vec<CollectedMetrics>> {
        self.access()
            .access()
            .map(|x| {
                x.get(n)
                    .into_iter()
                    .cloned()
                    .collect()
            })
    }
}

lazy_static! {
    pub static ref METRICS: MetricProvider = MetricProvider::default();
    pub static ref EVENTS: MetricsEvents = MetricsEvents::default();
}
