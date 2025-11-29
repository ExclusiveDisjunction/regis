use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use crate::loc::CLIENTS_PORT;
use exdisj::io::config::ConfigurationProvider;
use common::metric::Utilization;

use std::fmt::Display;
use std::net::IpAddr;

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

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub port: u16,
    pub cpu_warn: Utilization,
    pub cup_err: Utilization,
    pub mem_warn: Utilization,
    pub mem_err: Utilization,
    pub hosts: Vec<KnownHost>
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            port: CLIENTS_PORT,
            cpu_warn: Utilization::new_unwrap(70),
            cup_err: Utilization::new_unwrap(90),
            mem_warn: Utilization::new_unwrap(70),
            mem_err: Utilization::new_unwrap(90),
            hosts: vec![]
        }
    }
}
impl Configuration {}

lazy_static! {
    pub static ref CONFIG: ConfigurationProvider<Configuration> = ConfigurationProvider::default();
}