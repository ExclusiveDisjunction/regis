use std::fmt::{Debug, Display};
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
            write!(f, "{:.3} {}", self.amount, self.bracket)
        } else {
            write!(f, "{:.3} {}s", self.amount, self.bracket)
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
    pub inner: u8,
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

    pub fn take(self) -> u8 {
        self.into()
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
impl From<Utilization> for u8 {
    fn from(value: Utilization) -> Self {
        value.inner
    }
}

/// A general form of a data type that can be printed with specific index & tab offset values.
pub trait PrettyPrinter {
    fn pretty_print(&self, tabs: u8, index: Option<usize>) -> String;
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
            (BinaryNumber::parse(10), "10.000 bytes", "10 B"),
            (BinaryNumber::parse(2u64.pow(10)), "1.000 kibibyte", "1 KiB"),
            (BinaryNumber::parse(2u64.pow(10) * 2), "2.000 kibibytes", "2 KiB"),
            (BinaryNumber::parse(2u64.pow(20)), "1.000 mebibyte", "1 MiB"),
            (BinaryNumber::parse(2u64.pow(20) * 2), "2.000 mebibytes", "2 MiB"),
            (BinaryNumber::parse(2u64.pow(30)), "1.000 gibibyte", "1 GiB"),
            (BinaryNumber::parse(2u64.pow(30) * 2), "2.000 gibibytes", "2 GiB"),
            (BinaryNumber::parse(2u64.pow(40)), "1.000 tebibyte", "1 TiB"),
            (BinaryNumber::parse(2u64.pow(40) * 2), "2.000 tebibytes", "2 TiB"),
            (BinaryNumber::parse(2u64.pow(50)), "1.000 pebibyte", "1 PiB"),
            (BinaryNumber::parse(2u64.pow(50) * 2), "2.000 pebibytes", "2 PiB"),
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
