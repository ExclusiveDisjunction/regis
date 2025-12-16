
pub use common::metric::{MemoryMetric, NetworkMetric, CpuMetric, CollectedMetrics, StorageMetric};
use chrono::Utc;

pub(crate) trait MetricsCollector {
    async fn memory() -> Vec<MemoryMetric>;
    async fn network() -> Vec<NetworkMetric>;
    async fn cpu() -> Option<CpuMetric>;
    async fn storage() -> Vec<StorageMetric>;

    async fn collect() -> CollectedMetrics {
        CollectedMetrics {
            time: Utc::now(),
            memory: Self::memory().await,
            storage: Self::storage().await,
            cpu: Self::cpu().await,
            network: Self::network().await
        }
    }
}
