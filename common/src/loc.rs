pub const TOTAL_DIR: &str = "/etc/regis/";
pub const DAEMON_DIR: &str = "/etc/regis/regisd";
pub const DAEMON_CONFIG_PATH: &str = "/etc/regis/regisd/config.json";
pub const DAEMON_AUTH_DIR: &str = "/etc/regis/regisd/auth/";
pub const DAEMON_AUTH_USERS_PATH: &str = "/etc/regis/regisd/auth/users.json";
pub const DAEMON_AUTH_KEY_PATH: &str = "/etc/regis/regisd/auth/key";
pub const PID_PATH: &str = "/etc/regis/regisd/pid";
pub const COMM_DIR: &str = "/run/regis/";
pub const COMM_PATH: &str = "/run/regis/regis.sock";

/// Represents the default hosts port used by regis.
pub const CLIENTS_PORT: u16 = 1026;
pub const BROADCAST_PORT: u16 = 1027;

use std::path::PathBuf;
use std::env;

#[allow(unused_imports)]
use crate::err::{IO_ERR_EXIT, WEIRD_ERR_EXIT};

pub fn get_client_dir() -> PathBuf {
    let fragment: PathBuf;

    #[cfg(unix)] 
    {
        let home = match env::var("HOME") {
            Ok(v) => v,
            Err(e) => {
                panic!("Unable to get home directory '{e}'");
            }
        };

        fragment = PathBuf::from(home).join(".local").join("share");
    }

    #[cfg(target_os = "windows")] 
    {
        let home = match env::var("LOCALAPPDATA") {
            Ok(v) => v,
            Err(e) => {
                panic!("Unable to get local app data directory '{e}'");
            }
        };

        fragment = PathBuf::from(home).join("regis");
    }

    #[cfg(not(any(unix, windows)))]
    {
        panic!("Unfortunatley, the program cannot run on this platform, as there is no where to place the hosting directory.");
    }

    fragment.join("regis")
}
pub fn get_config_path() -> PathBuf {
    get_client_dir().join("config.json")
}
