use exdisj::error::RangeError;
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use crate::loc::CLIENTS_PORT;
use exdisj::config::{ConfigBase, ConfigurationProvider};
use common::metric::Utilization;

use std::fmt::Display;
use std::net::IpAddr;
use std::ffi::{c_ushort, c_uchar};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct KnownHost {
    addr: IpAddr,
    name: String
}
impl Display for KnownHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Host '{}' at IP '{}'", &self.name, &self.addr)
    }
}
impl KnownHost {
    pub fn new(name: String, addr: IpAddr) -> Self {
        Self {
            name: name.trim().to_string(),
            addr
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    pub fn addr(&self) -> &IpAddr {
        &self.addr
    }
    pub fn addr_mut(&mut self) -> &mut IpAddr {
        &mut self.addr
    }
}

pub mod defaults {
    pub const CPU_WARN: u8 = 70;
    pub const CPU_ERROR: u8 = 90;
    pub const MEM_WARN: u8 = 70;
    pub const MEM_ERROR: u8 = 90;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CoreConfig {
    pub port: u16,
    pub cpu_warn: Utilization,
    pub cpu_err: Utilization,
    pub mem_warn: Utilization,
    pub mem_err: Utilization
}
impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            port: CLIENTS_PORT,
            cpu_warn: Utilization::new_unwrap(defaults::CPU_WARN),
            cpu_err: Utilization::new_unwrap(defaults::CPU_ERROR),
            mem_warn: Utilization::new_unwrap(defaults::MEM_WARN),
            mem_err: Utilization::new_unwrap(defaults::MEM_ERROR),
        }
    }
}
impl ConfigBase for CoreConfig {}
impl CoreConfig {}

#[repr(C)]
pub struct BridgeConfig {
    pub port: c_ushort,
    pub cpu_warn: c_uchar,
    pub cpu_err: c_uchar,
    pub mem_warn: c_uchar,
    pub mem_err: c_uchar
}
impl TryFrom<BridgeConfig> for CoreConfig {
    type Error = RangeError<u8>;
    fn try_from(value: BridgeConfig) -> Result<Self, Self::Error> {
        Ok(
            CoreConfig {
                port: value.port,
                cpu_warn: Utilization::new(value.cpu_warn)?,
                cpu_err: Utilization::new(value.cpu_err)?,
                mem_warn: Utilization::new(value.mem_warn)?,
                mem_err: Utilization::new(value.mem_err)?
            }
        )
    }
}

lazy_static! {
    pub static ref CONFIG: ConfigurationProvider<CoreConfig> = ConfigurationProvider::default();
}