
#[cfg(target_os = "macos")]
pub const SERVER_COMM_DIR: &str = "/var/run/regisd";
#[cfg(target_os = "macos")]
pub const SERVER_COMM_PATH: &str = "/var/run/regisd/console.sock";

#[cfg(target_os="linux")]
pub const SERVER_COMM_PATH: &str = "/run/regisd/console.sock";
#[cfg(target_os = "linux")]
pub const SERVER_COMM_DIR: &str = "/run/regisd";