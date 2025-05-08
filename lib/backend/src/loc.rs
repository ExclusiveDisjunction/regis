use std::path::PathBuf;
use std::env;

pub use common::loc::*;

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
pub fn get_log_dir() -> PathBuf {
    get_client_dir().join("log")
}
pub fn get_config_path() -> PathBuf {
    get_client_dir().join("config.json")
}