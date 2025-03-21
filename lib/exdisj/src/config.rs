
use serde::{Serialize, Deserialize, ser::Error};
use serde_json::{from_str, to_string_pretty};

use std::{
    io::{
        Read,
        Write
    },
    fs::File,
    path::Path,
    sync::{
        Arc,
        RwLock
    },
    fmt::Debug
};

use crate::{
    lock::{
        OptionRwProvider,
        ProtectedAccess, 
        RwProvider
    },
    error::ParsingError
};

/// Represents the a specific set of configurations that can be stored in a file, and later retreived. 
pub trait ConfigBase: Serialize + for <'a> Deserialize<'a> + Debug { }

/// A structure that can be stored in a static variable, and provides configuration access. This follows the provider pattern.
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
impl<T> RwProvider for ConfigurationProvider<T> where T: ConfigBase {
    type Data = Option<T>;
    fn access_raw(&self) -> crate::lock::ProtectedAccess<'_, Arc<RwLock<Self::Data>>> {
        ProtectedAccess::new(&self.data)
    }
}
impl<T> OptionRwProvider<T> for ConfigurationProvider<T> where T: ConfigBase { }
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
}