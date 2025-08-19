use std::fmt::{Debug, Display};
use serde::{Serialize, Deserialize};

pub use exdisj::io::metric::{Utilization, BinaryNumber, BinaryScale, PrettyPrinter};

pub trait Metric: PartialEq + Debug + Clone + Serialize { }

/// Represents a collection of metrics that are taken at the same time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot<T>
where
    T: Metric,
{
    pub metrics: Vec<T>,
}
impl<T> PartialEq for Snapshot<T>
where
    T: Metric,
{
    fn eq(&self, other: &Self) -> bool {
        self.metrics.eq(&other.metrics)
    }
}
impl<T> Metric for Snapshot<T> where T: Metric {}
impl<T> Snapshot<T>
where
    T: Metric,
{
    pub fn new(metrics: Vec<T>) -> Self {
        Self { metrics }
    }
}
impl<T> PrettyPrinter for Snapshot<T> where T: Metric + PrettyPrinter {
    fn pretty_print(&self, tabs: u8, _index: Option<usize>) -> String {
        let mut result = String::new();
        for (i, metric) in self.metrics.iter().enumerate(){
            result += &metric.pretty_print(tabs, Some(i+1));
            result.push('\n');
            result.push('\n')

        }

        result
    }
}

/// Stores the information about a specific memory section. 
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct MemoryMetric {
    /// The name of the memory location
    pub name: String,
    /// The total space of that location
    pub total: BinaryNumber,
    /// The used space of that location
    pub used: BinaryNumber,
    /// The free space of the location
    pub free: BinaryNumber,
    pub shared: Option<BinaryNumber>,
    pub buff: Option<BinaryNumber>,
    pub available: Option<BinaryNumber>,
}
impl Metric for MemoryMetric {}
impl PrettyPrinter for MemoryMetric {
    fn pretty_print(&self, tabs: u8, index: Option<usize>) -> String {
        let tabs = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs;

        let header = if let Some(i) = index {
            format!("{}) ", i)
        }
        else {
            String::new()
        };

        let first_part = format!("{tabs_ref}{header}Name: '{}'\n{tabs_ref}Used: {}/{} ({} free)", &self.name, &self.used, &self.total, &self.free);

        if let (Some(shared), Some(buff), Some(ava)) = (self.shared.as_ref(), self.buff.as_ref(), self.available.as_ref()) {
            format!("{first_part}\n{tabs_ref}Shared: {}\n{tabs_ref}Buffer: {}\n{tabs_ref}Available: {}", shared, buff, ava)
        }
        else {
            first_part
        }
    }
}

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
impl PrettyPrinter for StorageMetric {
    fn pretty_print(&self, tabs: u8, index: Option<usize>) -> String {
        let tabs = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs;

        let header = if let Some(i) = index {
            format!("{}) ", i)
        }
        else {
            String::new()
        };

        format!("{tabs_ref}{header}Filesystem '{}' mounted at '{}'\n{tabs_ref}Used/Total: {}/{} ({} or {} free)", &self.system, &self.mount, &self.used, &self.size, &self.availiable, &self.capacity)
    }
}

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
    /// The time the processor spends handling hardware interupts
    pub h_interupts: u16,
    /// The time the processor spends handling software interupts
    pub s_interupts: u16,
    /// The time the processor spends handling virtual environments 
    pub steal: u16
}
impl Metric for CpuMetric {}
impl PrettyPrinter for CpuMetric {
    fn pretty_print(&self, tabs: u8, index: Option<usize>) -> String {
        let tabs = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs;

        let header = if let Some(i) = index {
            format!("{}) ", i)
        }
        else {
            String::new()
        };

        format!("{tabs_ref}{header}Utilization: {} user, {} system, {} priviledged ({} idle)\n{tabs_ref}IO Wait: {}s\n{tabs_ref}Time spent on interupts: {}s hardware, {}s software\n{tabs_ref}Stolen time: {}s", &self.user, &self.system, &self.nice, &self.idle, &self.waiting, &self.h_interupts, &self.s_interupts, &self.steal)
    }
}

/// Stores information about how many processes are running.
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ProcessCount {
    /// The number of running processes
    pub count: u64
}
impl Metric for ProcessCount {}
impl PrettyPrinter for ProcessCount {
    fn pretty_print(&self, tabs: u8, _index: Option<usize>) -> String {
        format!("{}Processes Running: {}", "\t".repeat(tabs as usize), &self.count)
    }
}

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
impl Display for NetworkMetricSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Ok: {}, Error: {}, Dropped: {}, Overrun: {}", 
            self.ok, self.err, self.drop, self.overrun
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
impl PrettyPrinter for NetworkMetric {
    fn pretty_print(&self, tabs: u8, index: Option<usize>) -> String {
        let tabs_string = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs_string;

        let header = if let Some(i) = index {
            format!("{}) ", i)
        }
        else {
            String::new()
        };

        format!("{tabs_ref}{header}Link Name: {}\n{tabs_ref}MTU: {}\n{tabs_ref}Receiving: {}\n{tabs_ref}Sending: {}", &self.name, &self.mtu, &self.rx, &self.tx)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct CollectedMetrics {
    pub time: i64,
    pub memory: Option<MemorySnapshot>,
    pub storage: Option<StorageSnapshot>,
    pub cpu: Option<CpuMetric>,
    pub network: Option<NetworkSnapshot>,
    pub proc_count: Option<ProcessCount>
}
impl PrettyPrinter for CollectedMetrics {
    fn pretty_print(&self, tabs: u8, index: Option<usize>) -> String {
        let tabs_string = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs_string;
        let tabs_string_one = "\t".repeat((tabs + 1) as usize);
        let empty_data = format!("{}No data measured", &tabs_string_one);

        let time = chrono::DateTime::from_timestamp(self.time, 0).map(|x| x.to_string()).unwrap_or("(Unknown Date/Time)".to_string());

        let cpu = self.cpu.as_ref().map(|x| x.pretty_print(tabs+1, None)).unwrap_or(empty_data.clone());
        let memory = self.memory.as_ref().map(|x| x.pretty_print(tabs + 1, None)).unwrap_or(empty_data.clone());
        let storage = self.storage.as_ref().map(|x| x.pretty_print(tabs + 1, None)).unwrap_or(empty_data.clone());
        let network = self.network.as_ref().map(|x| x.pretty_print(tabs + 1, None)).unwrap_or(empty_data.clone());
        let proc_count = self.proc_count.as_ref().map(|x| x.pretty_print(tabs, None)).unwrap_or(empty_data.clone());

        let header = if let Some(i) = index {
            format!("{}) ", i)
        }
        else {
            String::new()
        };
        
        format!("{tabs_ref}{header}Metrics at {time}:{tabs_ref}\n   CPU:\n{cpu}\n\n\n{tabs_ref}   Memory:\n{memory}\n{tabs_ref}   Storage:\n{storage}\n{tabs_ref}   Network:\n{network}\n   {proc_count}")
    }
}

pub type MemorySnapshot = Snapshot<MemoryMetric>;
pub type StorageSnapshot = Snapshot<StorageMetric>;
pub type NetworkSnapshot = Snapshot<NetworkMetric>;