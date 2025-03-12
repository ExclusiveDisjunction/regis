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
pub enum StorageDenom {
    Byte = 0,
    KiB = 1,
    MiB = 2,
    GiB = 3,
    TiB = 4,
    PiB = 5,
}
impl Debug for StorageDenom {
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
impl Display for StorageDenom {
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
impl TryFrom<String> for StorageDenom {
    type Error = ();
    /// Attempts to parse from shorthand, looking for either _iB, _B, _ib, _b, _i, or just B, b
    fn try_from(value: String) -> Result<Self, Self::Error> {
        <Self as TryFrom<&str>>::try_from(&value)
    }
}
impl TryFrom<&str> for StorageDenom {
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
impl StorageDenom {
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

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct StorageNum {
    amount: f64,
    bracket: StorageDenom,
}
impl Debug for StorageNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?}", self.amount, self.bracket)
    }
}
impl Display for StorageNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.amount == 1.0f64 {
            write!(f, "{} {}", self.amount, self.bracket)
        } else {
            write!(f, "{} {}s", self.amount, self.bracket)
        }
    }
}
impl StorageNum {
    pub fn new(amount: f64, bracket: StorageDenom) -> Self {
        Self { amount, bracket }
    }
    pub fn parse(raw: u64) -> Self {
        let bracket = StorageDenom::parse(raw);
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

pub trait Metric: PartialEq + Debug + Clone + Serialize {}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct MemoryMetric {
    name: String,
    total: StorageNum,
    used: StorageNum,
    free: StorageNum,
    shared: Option<StorageNum>,
    buff: Option<StorageNum>,
    available: Option<StorageNum>,
}
impl Metric for MemoryMetric {}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct StorageMetric {
    system: String,
    mount: String,
    size: StorageNum,
    used: StorageNum,
    capacity: Utilization,
}
impl Metric for StorageMetric {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot<T>
where
    T: Metric,
{
    metrics: Vec<T>,
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

pub type MemorySnapshot = Snapshot<MemoryMetric>;
pub type StorageSnapshot = Snapshot<StorageMetric>;

pub fn remove_blanks<'a>(on: Vec<&'a str>) -> Vec<&'a str> {
    let mut result: Vec<&'a str> = vec![];

    for item in on {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }

        result.push(trimmed)
    }

    result
}

pub async fn collect_memory() -> Option<MemorySnapshot> {
    if cfg!(target_os = "linux") {
        let output = Command::new("free").arg("-b").output().await.ok()?;

        if !output.status.success() {
            return None;
        }

        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        let by_line: Vec<String> = raw.split("\n").map(String::from).collect();

        /*
           Output pattern
           _        total    used  free   shared    buff    available
           [Name]: [total] [used] [free] [shared] [buff] [availiable]
        */

        if by_line.len() <= 1 {
            return None; //only the header was printed???
        }

        let mut list: Vec<MemoryMetric> = vec![];
        for line in by_line.into_iter().skip(1) {
            let cols = line.split(" ").collect();
            let fixed = remove_blanks(cols);

            //This is a greedy approach. It will attempt to fill as much as possible.
            //The first four are required

            if fixed.len() < 4 {
                return None; //Invalid length
            }

            let mut iter = fixed.into_iter();
            let name = iter.next()?;

            let mut converted = iter
                .map(|x| x.parse::<u64>().ok())
                .map(|x| x.map(StorageNum::parse));

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn storage_denom() {
        let values = vec![
            (10, StorageDenom::Byte),
            (1024, StorageDenom::KiB),
            (2u64.pow(20) + 4, StorageDenom::MiB),
            (2u64.pow(30) + 100, StorageDenom::GiB),
            (2u64.pow(40), StorageDenom::TiB),
            (2u64.pow(50), StorageDenom::PiB),
            (0, StorageDenom::Byte),
        ];

        for (i, (raw, expected)) in values.into_iter().enumerate() {
            let converted = StorageDenom::parse(raw);
            assert_eq!(converted, expected, "at {i} failed");
        }
    }

    #[test]
    fn storage_formatting() {
        let values = vec![
            (StorageNum::parse(10), "10 bytes", "10 B"),
            (StorageNum::parse(2u64.pow(10)), "1 kibibyte", "1 KiB"),
            (StorageNum::parse(2u64.pow(10) * 2), "2 kibibytes", "2 KiB"),
            (StorageNum::parse(2u64.pow(20)), "1 mebibyte", "1 MiB"),
            (StorageNum::parse(2u64.pow(20) * 2), "2 mebibytes", "2 MiB"),
            (StorageNum::parse(2u64.pow(30)), "1 gibibyte", "1 GiB"),
            (StorageNum::parse(2u64.pow(30) * 2), "2 gibibytes", "2 GiB"),
            (StorageNum::parse(2u64.pow(40)), "1 tebibyte", "1 TiB"),
            (StorageNum::parse(2u64.pow(40) * 2), "2 tebibytes", "2 TiB"),
            (StorageNum::parse(2u64.pow(50)), "1 pebibyte", "1 PiB"),
            (StorageNum::parse(2u64.pow(50) * 2), "2 pebibytes", "2 PiB"),
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
            (["B", "b", "B", "b", "b"], StorageDenom::Byte),
            (["KiB", "KB", "kib", "kb", "ki"], StorageDenom::KiB),
            (["MiB", "MB", "mib", "mb", "mi"], StorageDenom::MiB),
            (["GiB", "GB", "gib", "gb", "gi"], StorageDenom::GiB),
            (["TiB", "TB", "tib", "tb", "ti"], StorageDenom::TiB),
            (["PiB", "PB", "pib", "pb", "pi"], StorageDenom::PiB),
        ];

        let mut result = [[true; 5]; 6];

        for (i, (raw, expected)) in values.into_iter().enumerate() {
            let iter = raw
                .into_iter()
                .map(StorageDenom::try_from)
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
