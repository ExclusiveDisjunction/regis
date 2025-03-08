
use serde::{Serialize, Deserialize, ser::Error};
use serde_json::{from_str, to_string_pretty};
use lazy_static::lazy_static;

use std::io::{Read, Write};
use std::fs::File;
use std::net::IpAddr;
use std::path::Path;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::error::{ParsingError, PoisonError};
use crate::locations::{BROADCAST_PORT, CONSOLE_PORT, HOSTS_PORT};

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
impl Configuration {

}

pub struct ConfigReadGuard<'a> {
    inner: Result<RwLockReadGuard<'a, Option<Configuration>>, PoisonError>
}
impl<'a> From<RwLockReadGuard<'a, Option<Configuration>>> for ConfigReadGuard<'a> {
    fn from(value: RwLockReadGuard<'a, Option<Configuration>>) -> Self {
        Self {
            inner: Ok(value)
        }
    }
}
impl From<PoisonError> for ConfigReadGuard<'_> {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: Err(value)
        }
    }
}
impl<'a> ConfigReadGuard<'a> {
    pub fn access(&'a self) -> Option<&'a Configuration> {
        if let Ok(v) = self.inner.as_deref() {
            v.as_ref()
        }
        else {
            None
        }
    }
    pub fn access_error(&'a self) -> Option<&'a PoisonError> {
        self.inner.as_ref().err()
    }

    pub fn get_err(self) -> Option<PoisonError> {
        self.inner.err()
    }
    pub fn get_lock(self) -> Option<RwLockReadGuard<'a, Option<Configuration>>> {
        self.inner.ok()
    }
}

pub struct ConfigWriteGuard<'a> {
    inner: Result<RwLockWriteGuard<'a, Option<Configuration>>, PoisonError>
}
impl<'a> From<RwLockWriteGuard<'a, Option<Configuration>>> for ConfigWriteGuard<'a> {
    fn from(value: RwLockWriteGuard<'a, Option<Configuration>>) -> Self {
        Self {
            inner: Ok(value)
        }
    }
}
impl From<PoisonError> for ConfigWriteGuard<'_> {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: Err(value)
        }   
    }
}
impl<'a> ConfigWriteGuard<'a> {
    pub fn access(&'a mut self) -> Option<&'a mut Configuration> {
        if let Ok(v) = self.inner.as_deref_mut() {
            v.as_mut()
        }
        else {
            None
        }
    }
    pub fn access_error(&'a self) -> Option<&'a PoisonError> {
        self.inner.as_ref().err()
    }

    pub fn get_err(self) -> Option<PoisonError> {
        self.inner.err()
    }
    pub fn get_lock(self) -> Option<RwLockWriteGuard<'a, Option<Configuration>>> {
        self.inner.ok()
    }
}

pub struct ConfigurationProvider {
    data: Arc<RwLock<Option<Configuration>>>
}
impl Default for ConfigurationProvider {
    fn default() -> Self {
        Self {
            data: Arc::new(RwLock::new(None))
        }
    }
}
impl ConfigurationProvider {
    /// Reads the configuration file and returns any errors from IO or the parsing. 
    pub fn open<T: AsRef<Path>>(&self, path: T) -> Result<(), ParsingError> {
        let mut file = File::open(path).map_err(ParsingError::from)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(ParsingError::from)?;

        let result: Result<Configuration, _> = from_str(&contents);
        let result = match result {
            Ok(v) => v,
            Err(e) => return Err(e.into())
        };

        self.pass(result);
        Ok(())
    }
    pub fn pass(&self, config: Configuration) {
        let mut guard = match self.data.write() {
            Ok(g) => g,
            Err(e) => e.into_inner()
        };

        *guard = Some(config);
        self.data.clear_poison();
    }
    pub fn open_default(&self) {
        self.pass(Configuration::default())
    }
    /// Writes the configuration to the file system, and returns any conversions or IO errors.
    pub fn save<T: AsRef<Path>>(&self, path: T) -> Result<(), ParsingError> {
        let mut file = File::create(path).map_err(ParsingError::from)?;

        let guard = self.access();
        if let Some(v) = guard.access() {
            let contents = to_string_pretty(v).map_err(ParsingError::from)?;

            file.write_all(contents.as_bytes()).map_err(ParsingError::from)?;
            Ok(())
        }
        else {
            let err = guard.get_err().unwrap();
            Err(serde_json::Error::custom(err).into())
        }
    }
    pub fn set_to_default(&self) {
        {
            let mut lock = match self.data.write() {
                Ok(v) => v,
                Err(e) => e.into_inner()
            };
    
            *lock = Some(Configuration::default());
        }
        
        self.data.clear_poison();
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

    pub fn access(&self) -> ConfigReadGuard<'_> {
        match self.data.read() {
            Ok(v) => v.into(),
            Err(e) => PoisonError::new(&e).into()
        }
    }
    pub fn access_mut(&self) -> ConfigWriteGuard<'_> {
        match self.data.write() {
            Ok(v) => v.into(),
            Err(e) => PoisonError::new(&e).into()
        }
    }
}

lazy_static! {
    pub static ref CONFIG: ConfigurationProvider = ConfigurationProvider::default();
}