
use serde::{Serialize, Deserialize, ser::Error};
use serde_json::{from_str, to_string_pretty};

use std::io::{Read, Write};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::error::{ParsingError, PoisonError};

pub trait ConfigBase: Serialize + for <'a> Deserialize<'a> { }

pub struct ConfigReadGuard<'a, T> where T: ConfigBase {
    inner: Result<RwLockReadGuard<'a, Option<T>>, PoisonError>
}
impl<'a, T> From<RwLockReadGuard<'a, Option<T>>> for ConfigReadGuard<'a, T> where T: ConfigBase {
    fn from(value: RwLockReadGuard<'a, Option<T>>) -> Self {
        Self {
            inner: Ok(value)
        }
    }
}
impl<T> From<PoisonError> for ConfigReadGuard<'_, T> where T: ConfigBase {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: Err(value)
        }
    }
}
impl<'a, T> ConfigReadGuard<'a, T> where T: ConfigBase {
    pub fn access(&'a self) -> Option<&'a T> {
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
    pub fn get_lock(self) -> Option<RwLockReadGuard<'a, Option<T>>> {
        self.inner.ok()
    }
}

pub struct ConfigWriteGuard<'a, T> where T: ConfigBase {
    inner: Result<RwLockWriteGuard<'a, Option<T>>, PoisonError>
}
impl<'a, T> From<RwLockWriteGuard<'a, Option<T>>> for ConfigWriteGuard<'a, T> where T: ConfigBase{
    fn from(value: RwLockWriteGuard<'a, Option<T>>) -> Self {
        Self {
            inner: Ok(value)
        }
    }
}
impl<T> From<PoisonError> for ConfigWriteGuard<'_, T> where T: ConfigBase {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: Err(value)
        }   
    }
}
impl<'a, T> ConfigWriteGuard<'a, T> where T: ConfigBase {
    pub fn access(&'a mut self) -> Option<&'a mut T> {
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
    pub fn get_lock(self) -> Option<RwLockWriteGuard<'a, Option<T>>> {
        self.inner.ok()
    }
}

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
impl<T> ConfigurationProvider<T> where T: ConfigBase{
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
            let err = guard.get_err().unwrap();
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
        match self.data.read() {
            Ok(v) => v.into(),
            Err(e) => PoisonError::new(&e).into()
        }
    }
    pub fn access_mut(&self) -> ConfigWriteGuard<'_, T> {
        match self.data.write() {
            Ok(v) => v.into(),
            Err(e) => PoisonError::new(&e).into()
        }
    }
}