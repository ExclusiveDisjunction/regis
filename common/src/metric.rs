use std::{fmt::{Debug, Display}, io::empty};
use serde::{Serialize, Deserialize};

use crate::error::RangeError;

/// A measure that uses binary values for storage (GiB)
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BinaryScale {
    Byte = 0,
    KiB = 1,
    MiB = 2,
    GiB = 3,
    TiB = 4,
    PiB = 5,
}
impl Debug for BinaryScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Byte => "B",
                Self::KiB => "KiB",
                Self::MiB => "MiB",
                Self::GiB => "GiB",
                Self::TiB => "TiB",
                Self::PiB => "PiB",
            }
        )
    }
}
impl Display for BinaryScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Byte => "byte",
                Self::KiB => "kibibyte",
                Self::MiB => "mebibyte",
                Self::GiB => "gibibyte",
                Self::TiB => "tebibyte",
                Self::PiB => "pebibyte",
            }
        )
    }
}
impl TryFrom<String> for BinaryScale {
    type Error = ();
    /// Attempts to parse from shorthand, looking for either _iB, _B, _ib, _b, _i, or just B, b
    fn try_from(value: String) -> Result<Self, Self::Error> {
        <Self as TryFrom<&str>>::try_from(&value)
    }
}
impl TryFrom<&str> for BinaryScale {
    type Error = ();
    /// Attempts to parse from shorthand, looking for either _iB, _B, _ib, _b or just B, b
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value: String = value.trim().to_lowercase();

        if value == "b" {
            return Ok(Self::Byte);
        }

        let prefix = match value.strip_suffix("ib") {
            Some(v) => v,
            None => match value.strip_suffix("b") {
                Some(v) => v,
                None => match value.strip_suffix("i") {
                    Some(v) => v,
                    None => return Err(()),
                },
            },
        };

        let result = match prefix {
            "k" => Self::KiB,
            "m" => Self::MiB,
            "g" => Self::GiB,
            "t" => Self::TiB,
            "p" => Self::PiB,
            _ => return Err(()),
        };

        Ok(result)
    }
}
impl BinaryScale {
    pub fn parse(mut num: u64) -> Self {
        let mut power: u32 = 0;
        loop {
            if num < 2u64.pow(10) {
                break;
            }

            power += 1;
            num /= 2u64.pow(10);
        }

        match power {
            0 => Self::Byte,
            1 => Self::KiB,
            2 => Self::MiB,
            3 => Self::GiB,
            4 => Self::TiB,
            _ => Self::PiB,
        }
    }
}

/// Represents a number with a suffix, denoting scale. 
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BinaryNumber {
    amount: f64,
    bracket: BinaryScale,
}
impl Debug for BinaryNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?}", self.amount, self.bracket)
    }
}
impl Display for BinaryNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.amount == 1.0f64 {
            write!(f, "{} {}", self.amount, self.bracket)
        } else {
            write!(f, "{} {}s", self.amount, self.bracket)
        }
    }
}
impl BinaryNumber {
    pub fn new(amount: f64, bracket: BinaryScale) -> Self {
        Self { amount, bracket }
    }
    pub fn parse(raw: u64) -> Self {
        let bracket = BinaryScale::parse(raw);
        let mut value: f64 = raw as f64;

        let power = 2u64.pow(10 * (bracket as u32));
        value /= power as f64;

        Self {
            amount: value,
            bracket,
        }
    }

    pub fn in_bytes(self) -> u64 {
        let power = 2u64.pow(10 * (self.bracket as u32)) as f64;
        let product = self.amount * power;

        product as u64
    }
}

/// Represents a percentage between 0-100. 
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Utilization {
    inner: u8,
}
impl Utilization {
    pub fn new(raw: u8) -> Result<Self, RangeError<u8>> {
        if raw > 100 {
            Err(RangeError::new("inner", raw, Some((0, 100))))
        } else {
            Ok(Self { inner: raw })
        }
    }
    pub fn new_unwrap(raw: u8) -> Self {
        if raw > 100 {
            panic!("Index out of range. Utilization must be between 0-100")
        }

        Self { inner: raw }
    }
}
impl PartialOrd for Utilization {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Utilization {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}
impl Display for Utilization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.inner)
    }
}

pub trait PrettyPrinter {
    fn pretty_print(&self, tabs: u8) -> String;
}

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
    fn pretty_print(&self, tabs: u8) -> String {
        let mut result = String::new();
        for metric in &self.metrics {
            result += &metric.pretty_print(tabs);
            result.push('\n');

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
    fn pretty_print(&self, tabs: u8) -> String {
        let tabs = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs;

        if let (Some(shared), Some(buff), Some(ava)) = (self.shared.as_ref(), self.buff.as_ref(), self.available.as_ref()) {
            format!("{tabs_ref}Name: {}\n{tabs_ref}Used: {}/{} ({} free)\n{tabs_ref}Shared: {}\n{tabs_ref}Buffer: {}\n{tabs_ref}Available: {}", &self.name, &self.used, &self.total, &self.free, shared, buff, ava)
        }
        else {
            format!("{tabs_ref}Name: {}\n{tabs_ref}Used: {}/{} ({} free)", &self.name, &self.used, &self.total, &self.free)
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
    fn pretty_print(&self, tabs: u8) -> String {
        let tabs = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs;

        format!("{tabs_ref}Filesystem '{}' mounted at '{}'\n{tabs_ref}Used/Total: {}/{} ({} or {} free)", &self.system, &self.mount, &self.used, &self.size, &self.availiable, &self.capacity)
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
    fn pretty_print(&self, tabs: u8) -> String {
        let tabs = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs;

        format!("{tabs_ref}Utilization: {} user, {} system, {} priviledged ({} idle)\n{tabs_ref}IO Wait: {}s\n{tabs_ref}Time spent on interupts: {}s hardware, {}s software\n{tabs_ref}Stolen time: {}", &self.user, &self.system, &self.nice, &self.idle, &self.waiting, &self.h_interupts, &self.s_interupts, &self.steal)
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
    fn pretty_print(&self, tabs: u8) -> String {
        format!("{}Process Running: {}", "\t".repeat(tabs as usize), &self.count)
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
    fn pretty_print(&self, tabs: u8) -> String {
        let tabs_string = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs_string;

        format!("{tabs_ref}Link Name: {}\n{tabs_ref}MTU: {}\n{tabs_ref}Receiving: {}\n{tabs_ref}Sending: {}", &self.name, &self.mtu, &self.rx, &self.tx)
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
    fn pretty_print(&self, tabs: u8) -> String {
        let tabs_string = "\t".repeat(tabs as usize);
        let tabs_ref = &tabs_string;
        let tabs_string_one = "\t".repeat((tabs + 1) as usize);
        let empty_data = format!("{}No data measured", &tabs_string_one);

        let time = chrono::DateTime::from_timestamp(self.time, 0).map(|x| x.to_string()).unwrap_or("(Unknown Date/Time)".to_string());

        let cpu = self.cpu.as_ref().map(|x| x.pretty_print(tabs+1)).unwrap_or(empty_data.clone());
        let memory = self.memory.as_ref().map(|x| x.pretty_print(tabs + 1)).unwrap_or(empty_data.clone());
        let storage = self.storage.as_ref().map(|x| x.pretty_print(tabs + 1)).unwrap_or(empty_data.clone());
        let network = self.network.as_ref().map(|x| x.pretty_print(tabs + 1)).unwrap_or(empty_data.clone());
        let proc_count = self.proc_count.as_ref().map(|x| x.pretty_print(tabs + 1)).unwrap_or(empty_data.clone());
        
        format!("{tabs_ref}Metrics at {time}:{tabs_ref}  CPU:\n{cpu}\n{tabs_ref}   Memory:\n{memory}\n{tabs_ref}   Storage:\n{storage}\n{tabs_ref}\n{tabs_ref}   Network:\n{network}\n\n{proc_count}")
    }
}

pub type MemorySnapshot = Snapshot<MemoryMetric>;
pub type StorageSnapshot = Snapshot<StorageMetric>;
pub type NetworkSnapshot = Snapshot<NetworkMetric>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn storage_denom() {
        let values = vec![
            (10, BinaryScale::Byte),
            (1024, BinaryScale::KiB),
            (2u64.pow(20) + 4, BinaryScale::MiB),
            (2u64.pow(30) + 100, BinaryScale::GiB),
            (2u64.pow(40), BinaryScale::TiB),
            (2u64.pow(50), BinaryScale::PiB),
            (0, BinaryScale::Byte),
        ];

        for (i, (raw, expected)) in values.into_iter().enumerate() {
            let converted = BinaryScale::parse(raw);
            assert_eq!(converted, expected, "at {i} failed");
        }
    }

    #[test]
    fn storage_formatting() {
        let values = vec![
            (BinaryNumber::parse(10), "10 bytes", "10 B"),
            (BinaryNumber::parse(2u64.pow(10)), "1 kibibyte", "1 KiB"),
            (BinaryNumber::parse(2u64.pow(10) * 2), "2 kibibytes", "2 KiB"),
            (BinaryNumber::parse(2u64.pow(20)), "1 mebibyte", "1 MiB"),
            (BinaryNumber::parse(2u64.pow(20) * 2), "2 mebibytes", "2 MiB"),
            (BinaryNumber::parse(2u64.pow(30)), "1 gibibyte", "1 GiB"),
            (BinaryNumber::parse(2u64.pow(30) * 2), "2 gibibytes", "2 GiB"),
            (BinaryNumber::parse(2u64.pow(40)), "1 tebibyte", "1 TiB"),
            (BinaryNumber::parse(2u64.pow(40) * 2), "2 tebibytes", "2 TiB"),
            (BinaryNumber::parse(2u64.pow(50)), "1 pebibyte", "1 PiB"),
            (BinaryNumber::parse(2u64.pow(50) * 2), "2 pebibytes", "2 PiB"),
        ];

        for (i, (num, long, short)) in values.into_iter().enumerate() {
            let longhand = format!("{}", &num);
            let shorthand = format!("{:?}", &num);

            assert_eq!(&longhand, long, "longhand failed at {i}");
            assert_eq!(&shorthand, short, "shorthand failed at {i}");
        }
    }

    #[test]
    fn denom_parsing() {
        let values = vec![
            (["B", "b", "B", "b", "b"], BinaryScale::Byte),
            (["KiB", "KB", "kib", "kb", "ki"], BinaryScale::KiB),
            (["MiB", "MB", "mib", "mb", "mi"], BinaryScale::MiB),
            (["GiB", "GB", "gib", "gb", "gi"], BinaryScale::GiB),
            (["TiB", "TB", "tib", "tb", "ti"], BinaryScale::TiB),
            (["PiB", "PB", "pib", "pb", "pi"], BinaryScale::PiB),
        ];

        let mut result = [[true; 5]; 6];

        for (i, (raw, expected)) in values.into_iter().enumerate() {
            let iter = raw
                .into_iter()
                .map(BinaryScale::try_from)
                .map(|x| x == Ok(expected))
                .enumerate();

            for (j, v) in iter {
                result[i][j] = v;
            }
        }

        for (i, row) in result.into_iter().enumerate() {
            for (j, v) in row.into_iter().enumerate() {
                assert!(v, "at {i}, {j}");
            }
        }
    }
}
