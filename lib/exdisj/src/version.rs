use std::{cmp::Ordering, fmt::{Debug, Display}, str::FromStr};
use serde::{Serialize, Deserialize};
use crate::error::FormattingError;

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct Version {
    major: u16,
    minor: u16,
    build: u16
}
impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self as &dyn Display).fmt(f)
    }
}
impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.build)
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        }
        else {
            match self.major.cmp(&other.major) {
                Ordering::Equal => {
                    //The first numbers are equal, so we compare the second numbers
                    match self.minor.cmp(&other.minor) {
                        Ordering::Equal => {
                            //Second numbers are equal, so we compare the revison.
                            self.build.cmp(&other.build)
                        },
                        x => x
                    }
                },
                x => x
            }
        }
    }
}
impl FromStr for Version {
    type Err = FormattingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = [String::new(), String::new(), String::new()];
        let mut i = 0usize;
        for l in s.chars() {
            match l {
                '.' => {
                    if i + 1 >= parts.len() { //Too many periods!
                        return Err(FormattingError::new(&s, "too many periods contained"));
                    }
                    i += 1;
                },
                _ if l.is_numeric() => {
                    parts[i].push(l);
                },
                _ => return Err(FormattingError::new(&s, format!("unrecognized token '{}'", l)))
            }
        }

        let major_try: Result<u16, _> = parts[0].parse();
        let minor_try: Result<u16, _> = parts[1].parse();
        let build_try: Result<u16, _> = parts[2].parse();

        match (major_try, minor_try, build_try) {
            (Ok(ma), Ok(mi), Ok(bd)) => Ok(Self::new(ma, mi, bd)),
            _ => Err(FormattingError::new(&s, "one or more sub-parts could not be expressed as a u16"))
        }
    }
}
impl Version {
    pub const fn new(major: u16, minor: u16, build: u16) -> Self {
        Self {
            major,
            minor,
            build
        }
    }
}

#[test]
pub fn test_version_functions() {
    let v1 = Version::new(1, 0, 0);
    let v2 = Version::new(1, 3, 4);
    let v3 = Version::new(1, 3, 5);
    let v4 = Version::new(1, 0, 1);
    let v5 = Version::new(0, 1, 4);
    let v6 = Version::new(1, 1, 4);
    let v7 = Version::new(2, 0, 0);

    assert_eq!(format!("{v1}"), "1.0.0");
    assert!(v1 < v2 && v2 < v3, "v1 !< v2 || v2 !<v3");
    assert!(v1 < v4, "v1 !< v4");
    assert!(v5 < v6, "v5 !< v6");

    assert!(v3 > v1, "v3 !> v1");
    assert!(v2 > v1, "v2 !> v1");
    assert!(v3 > v5, "v3 !> v5");
    assert!(v7 > v1, "v7 !> v1");

    assert_eq!(v1, v1, "eq failed!");

    let mut list = vec![v1, v2, v3, v4, v5, v6, v7];
    list.sort();
    assert_eq!(list, vec![v5, v1, v4, v6, v2, v3, v7]);

    let v1_str: String = v1.to_string();
    let v1_decoded: Result<Version, _> = v1_str.parse();
    assert_eq!(v1_decoded.unwrap(), v1);

    let dummy_decoded: Result<Version, _> = ".4.0".parse();
    assert!(dummy_decoded.is_err());
}