use lazy_static::lazy_static;

use common::config::DaemonConfig;
use exdisj::io::config::ConfigurationProvider;

lazy_static! {
    pub static ref CONFIG: ConfigurationProvider<DaemonConfig> = ConfigurationProvider::default();
}
