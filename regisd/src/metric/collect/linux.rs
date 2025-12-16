use super::prelude::*;

pub struct LinuxCollector;
impl MetricsCollector for LinuxCollector {
     async fn cpu() -> Option<CpuMetric> {
        todo!()
     }
     async fn memory() -> Vec<MemoryMetric> {
        todo!() 
     }
     async fn network() -> Vec<NetworkMetric> {
        todo!()
     }
     async fn storage() -> Vec<StorageMetric> {
        todo!()
     }
}
