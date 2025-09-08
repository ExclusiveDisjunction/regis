use serde::{Serialize, Deserialize};

use crate::loc::CLIENTS_PORT;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Configuration {
    pub max_console: u8,
    pub max_hosts: u8,
    pub hosts_port: u16,
    pub metric_freq: u64,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            max_console: 4,
            max_hosts: 6,
            hosts_port: CLIENTS_PORT,
            metric_freq: 3,
        }
    }
}