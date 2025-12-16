use std::fmt::{Debug, Display};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

pub use exdisj::io::metric::{Utilization, BinaryNumber, BinaryScale, PrettyPrinter};

pub trait Metric: PartialEq + Debug + Clone + Serialize { }

/// Stores the information about a specific memory section. 
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct MemoryMetric {
    pub device: String,
    pub total: BinaryNumber,
    pub free: BinaryNumber,
    pub available: BinaryNumber,
    pub buff: BinaryNumber,
    pub cached: BinaryNumber
}
impl Metric for MemoryMetric {}

/// Stores the information about a specific storage section.
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct StorageMetric {
    /// The filesystem's name
    pub system: String,
    /// The mount point
    pub mount: String,
    /// The total size
    pub size: BinaryNumber,
    /// The used space
    pub used: BinaryNumber,
    /// How much space is availiable
    pub availiable: BinaryNumber,
    /// The utilization of the drive
    pub capacity: Utilization,
}
impl Metric for StorageMetric {}

/// Stores the information about CPU utilization
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct CpuMetric {
    /// How much of the processor is used for user processes
    pub user: Utilization,
    /// How much of the processor is used for system processes
    pub system: Utilization,
    /// How much of the processor is used for elevated processes
    pub nice: Utilization,
    /// How much of the processor is being unused.
    pub idle: Utilization,
    /// The time the processor spends waiting for IO
    pub waiting: u16,
    /// The time the processor spends handling virtual environments 
    pub steal: u16
}
impl Metric for CpuMetric {}

/// Stores the information for either the receive or transmitting section of the network. 
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct NetworkMetricSection {
    /// How mahy packets were OK
    pub ok: u64,
    /// How many packets had errors
    pub err: u64,
    /// How many packets were dropped
    pub drop: u64,
    /// How many packets were queued
    pub overrun: u64
}
impl TryFrom<Vec<u64>> for NetworkMetricSection {
    type Error = ();
    /// Attempts to build this structure from raw values. It returns error if there is not at least 4 elements. 
    fn try_from(value: Vec<u64>) -> Result<Self, Self::Error> {
        let mut iter = value.into_iter();
        Ok(
            Self {
                ok: iter.next().ok_or(())?,
                err: iter.next().ok_or(())?,
                drop: iter.next().ok_or(())?,
                overrun: iter.next().ok_or(())?
            }
        )
    }
}

/// Represents a snapshot of network activity through one specific link.
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct NetworkMetric {
    /// The name of the link
    pub name: String,
    /// MTU value
    pub mtu: String,
    /// The receiving information
    pub rx: NetworkMetricSection,
    /// The sending information 
    pub tx: NetworkMetricSection
}
impl Metric for NetworkMetric { }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct CollectedMetrics {
    pub time: DateTime<Utc>,
    pub memory: Vec<MemoryMetric>,
    pub storage: Vec<StorageMetric>,
    pub cpu: Option<CpuMetric>,
    pub network: Vec<NetworkMetric>,
}

const TAB1: &str = "\t";

pub struct CollectedMetricsFormatter<'a>(&'a CollectedMetrics);
impl Display for CollectedMetricsFormatter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Metrics at time {}", self.0.time)?;

        if !self.0.memory.is_empty() {
            Self::fmt_memory(f, &self.0.memory)?;
        }
        if !self.0.storage.is_empty() {
            Self::fmt_storage(f, &self.0.storage)?;
        }
        if let Some(cpu) = self.0.cpu.as_ref() {
            Self::fmt_cpu(f, cpu)?;
        }
        if !self.0.network.is_empty() {
            Self::fmt_network(f, &self.0.network)?;
        }

        Ok( () )
    }
}
impl<'a> CollectedMetricsFormatter<'a> {
    fn fmt_memory(f: &mut std::fmt::Formatter<'_>, mem: &[MemoryMetric]) -> std::fmt::Result {
        writeln!(f, "Memory:\n")?;
        writeln!(f, "   DEVICE   |   TOTAL   |   FREE   |   AVAIL.   |   BUFFERED   |   CACHED   |")?;
        writeln!(f, "------------|-----------|----------|------------|--------------|------------|")?;
        for device in mem {
            writeln!(f, " {:^10} | {:^9} | {:^8} | {:^10} | {:^12} | {:^10} |", &device.device, device.total, device.free, device.available, device.buff, device.cached)?;
        }

        Ok( () )
    }
    fn fmt_storage(f: &mut std::fmt::Formatter<'_>, storage: &[StorageMetric]) -> std::fmt::Result {
        writeln!(f, "Storage:\n")?;
        writeln!(f, "   DEVICE   |        MOUNT        |   SIZE   |    USED    |     AVAIL.   |   CAPACITY   |")?;
        writeln!(f, "------------|---------------------|----------|------------|--------------|--------------|")?;
        for device in storage {
            writeln!(f, " {:^10} | {:^19} | {:^8} | {:^8} | {:^10} | {:^12} |", &device.system, device.mount, device.size, device.used, device.availiable, device.capacity)?;
        }

        Ok( () )
    }
    fn fmt_cpu(f: &mut std::fmt::Formatter<'_>, cpu: &CpuMetric) -> std::fmt::Result {
        writeln!(f, "CPU:")?;
        writeln!(f, "{TAB1} User: {}", cpu.user)?;
        writeln!(f, "{TAB1} System: {}", cpu.system)?;
        writeln!(f, "{TAB1} Nice: {}", cpu.nice)?;
        writeln!(f, "{TAB1} Idle: {}", cpu.idle)?;
        writeln!(f, "{TAB1} IO Waiting: {}", cpu.waiting)?;
        writeln!(f, "{TAB1} Stolen Time: {}", cpu.steal)?;

        Ok( () )
    }
    fn fmt_network(f: &mut std::fmt::Formatter<'_>, network: &[NetworkMetric]) -> std::fmt::Result {
        writeln!(f, "Network:\n")?;
        writeln!(f, "Todo tee hee, print {} elements", network.len())?;

        Ok( () )
    }

    pub fn new(data: &'a CollectedMetrics) -> Self {
        Self(data)
    }
}
