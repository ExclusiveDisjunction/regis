use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;

use crate::locations::{BROADCAST_PORT, CONSOLE_PORT, HOSTS_PORT};
use common::config::{ConfigBase, ConfigurationProvider};

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub max_console: u8,
    pub max_hosts: u8,
    pub console_port: u16,
    pub hosts_port: u16,
    pub broadcasts_port: u16,
    pub metric_frec: u64
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            max_console: 4,
            max_hosts: 6,
            console_port: CONSOLE_PORT,
            hosts_port: HOSTS_PORT,
            broadcasts_port: BROADCAST_PORT,
            metric_frec: 1,
        }
    }
}
impl ConfigBase for Configuration { }
impl Configuration { }

lazy_static! {
    pub static ref CONFIG: ConfigurationProvider<Configuration> = ConfigurationProvider::default();
}