use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::locations::{BROADCAST_PORT, CLIENTS_PORT};
use common::config::{ConfigBase, ConfigurationProvider};

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub max_console: u8,
    pub max_hosts: u8,
    pub hosts_port: u16,
    pub broadcasts_port: u16,
    pub metric_freq: u64,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            max_console: 4,
            max_hosts: 6,
            hosts_port: CLIENTS_PORT,
            broadcasts_port: BROADCAST_PORT,
            metric_freq: 3,
        }
    }
}
impl ConfigBase for Configuration {}
impl Configuration {}

lazy_static! {
    pub static ref CONFIG: ConfigurationProvider<Configuration> = ConfigurationProvider::default();
}
