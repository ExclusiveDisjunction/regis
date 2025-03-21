
use serde::{Serialize, Deserialize, ser::Error};
use serde_json::{from_str, to_string_pretty};

use std::io::{Read, Write};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, RwLock};

use super::lock::{OptionReadGuard, OptionWriteGuard};

use crate::error::{ParsingError, PoisonError};

pub trait ConfigBase: Serialize + for <'a> Deserialize<'a> { }

pub type ConfigReadGuard<'a, T> = OptionReadGuard<'a, T>;
pub type ConfigWriteGuard<'a, T> = OptionWriteGuard<'a, T>;

pub struct ConfigurationProvider<T> where T: ConfigBase {
    data: Arc<RwLock<Option<T>>>
}
impl<T> Default for ConfigurationProvider<T> where T: ConfigBase {
    fn default() -> Self {
        Self {
            data: Arc::new(RwLock::new(None))
        }
    }
}
impl<T> ConfigurationProvider<T> where T: ConfigBase {
    /// Reads the configuration file and returns any errors from IO or the parsing. 
    pub fn open<P: AsRef<Path>>(&self, path: P) -> Result<(), ParsingError> {
        let mut file = File::open(path).map_err(ParsingError::from)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(ParsingError::from)?;

        let result: Result<T, _> = from_str(&contents);
        let result = match result {
            Ok(v) => v,
            Err(e) => return Err(e.into())
        };

        self.pass(result);
        Ok(())
    }
    pub fn pass(&self, config: T) {
        let mut guard = match self.data.write() {
            Ok(g) => g,
            Err(e) => e.into_inner()
        };

        *guard = Some(config);
        self.data.clear_poison();
    }
    pub fn set_to_default(&self) where T: Default {
        self.pass(T::default())
    }
    /// Writes the configuration to the file system, and returns any conversions or IO errors.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ParsingError> {
        let mut file = File::create(path).map_err(ParsingError::from)?;

        let guard = self.access();
        if let Some(v) = guard.access() {
            let contents = to_string_pretty(v).map_err(ParsingError::from)?;

            file.write_all(contents.as_bytes()).map_err(ParsingError::from)?;
            Ok(())
        }
        else {
            let err = guard.access_error().unwrap();
            Err(serde_json::Error::custom(err).into())
        }
    }

    pub fn close(&self) {
        match self.data.write() {
            Ok(mut v) => *v = None,
            Err(e) => {
                let mut inner = e.into_inner();
                *inner = None;
                self.data.clear_poison();
            }
        }
    }
    pub fn is_open(&self) -> bool {
        self.data
        .read()
        .map(|v| v.is_some()) 
        .ok()
        .unwrap_or(false)
    }
    pub fn is_poisoned(&self) -> bool {
        self.data.is_poisoned()
    }

    pub fn access(&self) -> ConfigReadGuard<'_, T> {
        self.data.read()
            .map_err(PoisonError::new)
            .into()
    }
    pub fn access_mut(&self) -> ConfigWriteGuard<'_, T> {
        self.data.write()
            .map_err(PoisonError::new)
            .into()
    }
}