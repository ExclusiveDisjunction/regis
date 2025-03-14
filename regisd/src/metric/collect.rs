/*
    Metric collection

    This module is used to handle metric parsing using specific commands.

    Memory
    CPU
    Network
    Disk Usage
*/

use std::fmt::{Debug, Display};
use tokio::process::Command;

use common::error::RangeError;
use serde::{Deserialize, Serialize};

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
impl Display for Utilization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.inner)
    }
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
    /// The time the processor spends handling hardware interupts
    pub h_interupts: u16,
    /// The time the processor spends handling software interupts
    pub s_interupts: u16,
    /// The time the processor spends handling virtual environments 
    pub steal: u16
}
impl Metric for CpuMetric {}

/// Stores information about how many processes are running.
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ProcessCount {
    /// The number of running processes
    pub count: u64
}
impl Metric for ProcessCount {}

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectedMetrics {
    pub time: i64,
    pub memory: Option<MemorySnapshot>,
    pub storage: Option<StorageSnapshot>,
    pub cpu: Option<CpuMetric>,
    pub network: Option<NetworkSnapshot>,
    pub proc_count: Option<ProcessCount>
}

pub type MemorySnapshot = Snapshot<MemoryMetric>;
pub type StorageSnapshot = Snapshot<StorageMetric>;
pub type NetworkSnapshot = Snapshot<NetworkMetric>;

pub async fn collect_memory() -> Option<MemorySnapshot> {
    if cfg!(target_os = "linux") {
        let output = Command::new("free").arg("-b").output().await.ok()?;

        if !output.status.success() {
            return None;
        }

        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        let by_line: Vec<&str> = raw.split("\n")
            .skip(1) //Skip the header
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .collect();

        /*
           Output pattern
           _        total    used  free   shared    buff    available
           [Name]: [total] [used] [free] [shared] [buff] [availiable]
        */

        if by_line.len() <= 1 {
            return None; //only the header was printed???
        }

        let mut list: Vec<MemoryMetric> = vec![];
        for line in by_line{
            let cols: Vec<&str> = line.split(" ")
                .map(|x| x.trim())
                .filter(|x| !x.is_empty())
                .collect();

            //This is a greedy approach. It will attempt to fill as much as possible.
            //The first four are required

            if cols.len() < 4 {
                return None; //Invalid length
            }

            let mut iter = cols.into_iter();
            let name = iter.next()?;

            let mut converted = iter
                .map(|x| x.parse::<u64>().ok())
                .map(|x| x.map(BinaryNumber::parse));

            list.push(MemoryMetric {
                name: name.to_string(),
                total: converted.next()??,
                used: converted.next()??,
                free: converted.next()??,
                shared: converted.next().unwrap_or(None),
                buff: converted.next().unwrap_or(None),
                available: converted.next().unwrap_or(None),
            })
        }

        Some(MemorySnapshot::new(list))
    } else {
        None
    }
}

pub async fn collect_storage() -> Option<StorageSnapshot> {
    if !cfg!(target_os="linux") {
        return None;
    }

    let output = Command::new("df")
        .arg("-BK")
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    let by_line = raw.split("\n")
        .skip(1) //Skip the header 
        .filter(|x| !x.trim().is_empty()); //Skips empty lines

    let mut result: Vec<StorageMetric> = vec![];
    for line in by_line {
        let splits: Vec<&str> = line.split(' ')
            .filter(|x| !x.trim().is_empty())
            .map(|x| x.trim())
            .filter(|x| x.starts_with("/dev"))
            .collect();

        //Len should be 6
        if splits.len() != 6 {
            continue;
        }

        let name = splits[0].to_owned();
        let disk_stats: Vec<BinaryNumber> = splits[1..3]
            .iter()
            .map(|x| {
                let raw = match x.strip_suffix("K") {
                    Some(v) => v,
                    None => x
                };

                let parsed: u64 = raw.parse().unwrap_or(0);
                BinaryNumber::parse(parsed)
            })
            .collect();
        let capacity = Utilization::new(splits[4].parse::<u8>().unwrap_or(0)).ok()?;
        let mounted = splits[5].to_owned();

        result.push(
            StorageMetric {
                system: name,
                mount: mounted,
                size: disk_stats[0],
                used: disk_stats[1],
                availiable: disk_stats[2],
                capacity,
            }
        )
    }

    todo!()
}

pub async fn collect_cpu() -> Option<CpuMetric> {
    if !cfg!(target_os="linux") {
        return None;
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg("top -b -n 1 | grep \"%(Cpu(s)\"")
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();

    let without_cpu = raw.strip_prefix("%Cpu(s): ")?;
    let comma_splits: Vec<&str> = without_cpu.split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect();

    /*  
        Format at this point: 
        [Value] [suffix]

        We need to remove the suffix. It always comes with a space after the value, so we can split by space, and only keep the first one.

        There should be exaclty 8 arguments.
     */
    
    let raw_values: Vec<&str> = comma_splits.into_iter()
        .map(|x| {
            x.split(' ').next() //Only takes the first value, if it exists
        })
        .filter(|x| x.is_some()) //Only keep the real values
        .map(|x| x.unwrap()) //Convert it from an option to a real value
        .filter(|x| !x.trim().is_empty())
        .collect(); //Remove the empty entries

    let parsed_values: Vec<u16>= raw_values.into_iter()
        .map(|x| x.parse::<f64>())
        .map(|x| {
            if let Ok(v) = x {
                Some(v as u16)
            }
            else {
                None
            }
        })
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect();

    if parsed_values.len() != 8 {
        return None;
    }

    //The first four are supposed to be utiliziations, the remainder are to be interpreted as u16 durations.

    let utils: Vec<Utilization> = parsed_values[0..3]
        .iter()
        .map(|x| *x as u8)
        .map(Utilization::new)
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect();

    if utils.len() != 4 {
        return None;
    }

    Some(
        CpuMetric {
            user: utils[0],
            system: utils[1],
            nice: utils[2],
            idle: utils[3],
            waiting: parsed_values[4],
            h_interupts: parsed_values[5],
            s_interupts: parsed_values[6],
            steal: parsed_values[7],
        }
    )
}

pub async fn collect_network() -> Option<NetworkSnapshot> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    let output = Command::new("netstat")
        .arg("-i")
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    /*

        The format is: 

        -Ignore Row-
        -Ignore Row-
        [Name] [Mtu] [RX..4] [TX..4] -Ignore-
     */

    let by_line: Vec<&str> = raw.split('\n')
        .skip(2) //Skip the headers
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect();

    let mut result: Vec<NetworkMetric> = vec![];
    for line in by_line {
        let splits: Vec<&str> = line.split(' ')
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .collect();

        if splits.len() != 11 {
            continue;
        }

        let name = splits[0].to_owned();
        let mtu = splits[1].to_owned();
        let rx_raw: Vec<&str> = splits[2..5].to_vec();
        let tx_raw: Vec<&str> = splits[6..9].to_vec();
        //The flag `splits[10]` is ignored

        let rx_values: Vec<u64> = rx_raw.into_iter()
            .map(|x| x.parse::<u64>().unwrap_or(0))
            .collect();
        let tx_values: Vec<u64> = tx_raw.into_iter()
            .map(|x| x.parse::<u64>().unwrap_or(0))
            .collect();

        let rx = NetworkMetricSection::try_from(rx_values).ok()?;
        let tx = NetworkMetricSection::try_from(tx_values).ok()?;

        result.push(
            NetworkMetric {
                name,
                mtu,
                rx,
                tx
            }
        )
    }   

    Some(
        NetworkSnapshot::new(result)
    )
}

pub async fn collect_process_count() -> Option<ProcessCount> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg("ps -e --no-headers | wc -l")
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    let amount: u64 = raw.parse().ok()?;

    Some(
        ProcessCount {
            count: amount
        }
    )
}

pub async fn collect_all_snapshots() -> CollectedMetrics {
    let time = chrono::Local::now().timestamp();

    CollectedMetrics {
        time,
        memory: collect_memory().await,
        storage: collect_storage().await,
        cpu: collect_cpu().await,
        network: collect_network().await,
        proc_count: collect_process_count().await
    }
}

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
