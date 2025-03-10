use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::ops::{Deref, DerefMut};

use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use crate::error::{IOError, OperationError};

/// Determines the level used by the logger
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum LoggerLevel {
    Debug = 1,
    Info = 2,
    Warning = 3,
    Error = 4,
    Critical = 5 
}
impl Debug for LoggerLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Debug => "DEBUG",
                Self::Info => "INFO",
                Self::Warning => "WARNING",
                Self::Error => "ERROR",
                Self::Critical => "CRITICAL"
            }
        )
    }
}

/// Abstraction for the logger to handle writing information to stdout and stderror.
pub struct LoggerRedirect {
    std_out: Option<LoggerLevel>,
    std_err: bool
}
impl Default for LoggerRedirect {
    fn default() -> Self {
        Self {
            std_out: None,
            std_err: true
        }
    }
}
impl LoggerRedirect {
    pub fn new(std_out: Option<LoggerLevel>, std_err: bool) -> Self {
        Self {
            std_out,
            std_err
        }
    }

    pub fn handle_redirect(&self, write: &LoggerWrite) {
        if self.std_err && (write.level() == LoggerLevel::Error || write.level() == LoggerLevel::Critical) {
            eprintln!("{}", write.contents())
        }

        if let Some(s) = self.std_out {
            if write.level() >= s {
                println!("{}", write.contents())
            }
        }
    }
}

/// A single write-in-progress for the logger
pub struct LoggerWrite {
    contents: String,
    level: LoggerLevel
}
impl LoggerWrite {
    pub fn blank(level: LoggerLevel) -> Self {
        Self {
            contents: String::new(),
            level
        }
    }
    pub fn new<T: Debug>(contents: &T, level: LoggerLevel) -> Self {
        Self {
            contents: format!("{:?}", contents),
            level
        }
    }
    pub fn new_str(contents: String, level: LoggerLevel) -> Self {
        Self {
            contents,
            level
        }
    }

    pub fn ignore(&self, level: LoggerLevel) -> bool {
        self.level < level
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }
    pub fn level(&self) -> LoggerLevel {
        self.level
    }
    pub fn append<T: Debug>(&mut self, cont: &T) {
        let new_cont: String = format!("{:?}", cont);
        self.contents += &new_cont;
    }
}

/// A structure that facilitates the writing done.
pub struct LoadedLogger {
    file: File,
    level: LoggerLevel,
    redirect: LoggerRedirect,
    write: Option<LoggerWrite>
}
impl LoadedLogger {
    pub fn new(file: File, level: LoggerLevel, redirect: LoggerRedirect) -> Self {
        Self {
            file,
            level,
            redirect,
            write: None
        }
    }

    pub fn level(&self) -> LoggerLevel {
        self.level
    }
    pub fn redirect(&self) -> &LoggerRedirect {
        &self.redirect
    }
    pub fn set_redirect(&mut self, new: LoggerRedirect) {
        self.redirect = new
    }

    pub fn is_writing(&self) -> bool {
        self.write.is_some()
    }
    pub fn writing_level(&self) -> Option<LoggerLevel> {
        let write = self.write.as_ref()?;
        Some(write.level())
    }
    pub fn current_log_ignored(&self) -> Option<bool> {
        let write = self.write.as_ref()?;
        Some( write.level() < self.level )
    }

    pub fn start_log(&mut self, level: LoggerLevel) -> Result<(), OperationError>{
        if self.is_writing() {
            return Err( OperationError::new("start log", format!("log already started at level {:?}", self.writing_level().unwrap())) );
        }

        let write = LoggerWrite::new(
            &format!("{:?} {:?}", chrono::Local::now(), level),
            level
        );

        self.write = Some(write);
        Ok(())
    }
    pub fn write<T: Debug>(&mut self, obj: &T) -> bool {
        let write = match self.write.as_mut() {
            Some(s) => s,
            None => return false
        };

        write.append(obj);
        true
    }
    pub fn end_log(&mut self) -> Result<(), IOError> {
        let write = self.write.as_ref().ok_or(IOError::Core( OperationError::new("end log", "no log was started").into() ))?;

        if !write.ignore(self.level) {
            let mut contents = write.contents().to_string();
            contents.push('\n');

            self.redirect.handle_redirect(write);

            self.file.write(contents.as_bytes()).map_err(IOError::from)?;
        }

        self.write = None;
        Ok(())
    }

    /// Regardless of a log being currently in progress or not, this will direclty write a string into the log file. 
    pub fn write_direct(&mut self, contents: String, level: LoggerLevel) -> Result<(), std::io::Error> {
        let write = LoggerWrite::new_str(
            format!("{:?} {:?} {}\n", chrono::Local::now(), level, contents),
            level
        );
        self.redirect.handle_redirect(&write);
        
        self.file.write_all(write.contents().as_bytes())?;
        Ok(())
    }
}

/// A simple structure to handle poison errors as None, and provides a wrapper around the mutex lock.
pub struct LoggerLock<'a> {
    inner: Option<MutexGuard<'a, Option<LoadedLogger>>>
}
impl<'a> LoggerLock<'a> {
    pub fn new(guard: Option<MutexGuard<'a, Option<LoadedLogger>>>) -> Self {
        Self {
            inner: guard
        }
    }
    pub fn new_poisioned() -> Self {
        Self {
            inner: None
        }
    }

    pub fn access(&self) -> Option<&LoadedLogger> {
        let acc = self.inner.as_ref()?;
        let x: &Option<LoadedLogger> = acc.deref();
        x.as_ref()
    }
    pub fn access_mut(&mut self) -> Option<&mut LoadedLogger> {
        let acc = self.inner.as_mut()?;
        let x: &mut Option<LoadedLogger> = acc.deref_mut();
        x.as_mut()
    }
    pub fn is_poisoned(&self) -> bool {
        self.inner.is_none()
    }
}

/// A global safe structure used to load and manage a logger.
pub struct Logger {
    data: Arc<Mutex<Option<LoadedLogger>>>
}
impl Default for Logger {
    fn default() -> Self {
        Self {
            data: Arc::new(Mutex::new(None))
        }
    }
}
impl Logger {
    pub fn open<T: AsRef<Path>>(&self, path: T, level: LoggerLevel, redirect: LoggerRedirect) -> Result<(), std::io::Error> {
        let file = File::create(path)?;

        let loaded = LoadedLogger::new(
            file,
            level,
            redirect
        );

        self.pass(loaded);
        Ok(())
    }
    pub fn pass(&self, logger: LoadedLogger) {
        let mut guard = match self.data.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner()
        };

        *guard = Some(logger);
        self.data.clear_poison(); //Since this function is always overriding the stored value, it is ok to clear the error.
    }
    pub fn close(&self) {
        match self.data.lock() {
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
        .lock()
        .map(|v| v.is_some())
        .ok()
        .unwrap_or(false)
    }
    pub fn is_poisoned(&self) -> bool {
        self.data.is_poisoned()
    }

    pub fn access(&self) -> LoggerLock<'_> {
        LoggerLock::new(self.data.lock().ok())
    }

    pub fn level(&self) -> Option<LoggerLevel> {
        let data = self.data.lock().unwrap();
        data.as_ref().map(|x| x.level())
    }
}

lazy_static! {
    pub static ref logging: Logger = Logger::default();
}

#[macro_export]
macro_rules! logger_write {
    ($level: expr, $($arg:tt)*) => {
        {
            if $crate::log::logging.is_open() { //Do nothing, so that standard error is not flooded with 'not open' errors.
                #[allow(unreachable_patterns)]
                let true_level: $crate::log::LoggerLevel = match $level {
                    $crate::log::LoggerLevel::Debug => $crate::log::LoggerLevel::Debug,
                    $crate::log::LoggerLevel::Info => $crate::log::LoggerLevel::Info,
                    $crate::log::LoggerLevel::Warning => $crate::log::LoggerLevel::Warning,
                    $crate::log::LoggerLevel::Error => $crate::log::LoggerLevel::Error,
                    $crate::log::LoggerLevel::Critical => $crate::log::LoggerLevel::Critical,
                    //_ => compile_error!("the type passed into this enum must be of LoggerLevel")
                };
                let mut aquired = $crate::log::logging.access();

                if aquired.access().map(|x| true_level >= x.level() && !x.is_writing()).unwrap_or(false) {
                    let contents: String = format!($($arg)*);

                    if let Some(cont) = aquired.access_mut() {
                        if let Err(e) = cont.write_direct(contents, true_level) {
                            eprintln!("unable to end log because of '{:?}'. Log will be closed", e);
                            $crate::log::logging.close();
                        }
                    }
                }
            }
        }
    };
}
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Debug, $($arg)*)
        }
    }
}
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Info, $($arg)*)
        }
    }
}
#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Warning, $($arg)*)
        }
    }
}
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Error, $($arg)*)
        }
    }
}
#[macro_export]
macro_rules! log_critical {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Critical, $($arg)*)
        }
    }
}

#[test]
fn test_logger_write() {
    if let Err(e) = logging.open("tmp.log", LoggerLevel::Debug, LoggerRedirect::default()) {
        panic!("unable to open log because '{:?}'", e);
    }

    logger_write!(LoggerLevel::Debug, "hello");
    logger_write!(LoggerLevel::Info, "hello");
    logger_write!(LoggerLevel::Warning, "hello");
    logger_write!(LoggerLevel::Error, "hello");
    logger_write!(LoggerLevel::Critical, "hello");

    log_debug!("hello 2");
    log_info!("hello 2");
    log_warning!("hello 2");
    log_error!("hello 2");
    log_critical!("hello 2");

    logging.close();
    assert!(!logging.is_open());
}