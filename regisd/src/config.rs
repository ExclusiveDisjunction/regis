use lazy_static::lazy_static;

use common::config::Configuration;
use exdisj::io::config::ConfigurationProvider;

lazy_static! {
    pub static ref CONFIG: ConfigurationProvider<Configuration> = ConfigurationProvider::default();
}
