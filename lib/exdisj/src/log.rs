use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use crate::error::{IOError, OperationError};
use crate::lock::{MutexProvider, OptionMutexProvider, ProtectedAccess};

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
#[derive(Debug, Clone)]
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
            eprintln!("{}", write.contents());
            return;
        }

        if let Some(s) = self.std_out {
            if write.level() >= s {
                println!("{}", write.contents())
            }
        }
    }
}

/// A single write-in-progress for the logger
#[derive(Debug, Clone)]
pub struct LoggerWrite {
    time_stamp: String,
    contents: String,
    level: LoggerLevel
}
impl LoggerWrite {
    pub fn blank(level: LoggerLevel, time_stamp: String) -> Self {
        Self {
            time_stamp,
            contents: String::new(),
            level
        }
    }
    pub fn new<T: Debug>(time_stamp: String, contents: &T, level: LoggerLevel) -> Self {
        Self {
            time_stamp,
            contents: format!("{:?}", contents),
            level
        }
    }
    pub fn new_str(time_stamp: String, contents: String, level: LoggerLevel) -> Self {
        Self {
            time_stamp,
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
impl From<LoggerWrite> for Vec<u8> {
    fn from(value: LoggerWrite) -> Self {
        format!("{} {:?} {}\n", value.time_stamp, value.level, value.contents).into_bytes()
    }
}

/// A structure that facilitates the writing done.
#[derive(Debug)]
pub struct LoadedLogger {
    file: File,
    level: LoggerLevel,
    redirect: LoggerRedirect,
    write: Option<LoggerWrite>
}
impl LoadedLogger {
    /// Initalizes the structure.
    pub fn new(file: File, level: LoggerLevel, redirect: LoggerRedirect) -> Self {
        Self {
            file,
            level,
            redirect,
            write: None
        }
    }

    /// Determines the level at which the logger is operating.
    pub fn level(&self) -> LoggerLevel {
        self.level
    }
    /// Determines the logger's redirect.
    pub fn redirect(&self) -> &LoggerRedirect {
        &self.redirect
    }
    /// Sets the logger's redirect.
    pub fn set_redirect(&mut self, new: LoggerRedirect) {
        self.redirect = new
    }

    /// Determines if the logger is currently writing a log value.
    pub fn is_writing(&self) -> bool {
        self.write.is_some()
    }
    /// Determines the level at which the current log is being written in.
    pub fn writing_level(&self) -> Option<LoggerLevel> {
        let write = self.write.as_ref()?;
        Some(write.level())
    }
    /// Determines if the current log, if being written, will actually be written to the file.
    pub fn current_log_ignored(&self) -> Option<bool> {
        let write = self.write.as_ref()?;
        Some( write.level() < self.level )
    }

    /// Prepares the logger for a new writing session.
    /// Fails if there is currently a log ongoing.
    pub fn start_log(&mut self, level: LoggerLevel) -> Result<(), OperationError>{
        if self.is_writing() {
            return Err( OperationError::new("start log", format!("log already started at level {:?}", self.writing_level().unwrap())) );
        }

        let write = LoggerWrite::new_str(
            format!("{}", chrono::Local::now()),
            String::new(),
            level
        );

        self.write = Some(write);
        Ok(())
    }
    /// Writes data into the current log, if one is active.
    pub fn write<T: Debug>(&mut self, obj: &T) -> bool {
        let write = match self.write.as_mut() {
            Some(s) => s,
            None => return false
        };

        write.append(obj);
        true
    }
    /// Flushes the internal buffer of the log, and marks the logger as being completed.
    pub fn end_log(&mut self) -> Result<(), IOError> {
        let write = self.write.as_ref().ok_or(IOError::Core( OperationError::new("end log", "no log was started").into() ))?;

        if !write.ignore(self.level) {
            let mut contents = write.time_stamp.clone() + write.contents();
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
            format!("{}", chrono::Local::now()),
            contents,
            level
        );
        self.redirect.handle_redirect(&write);
        let bytes: Vec<u8> = write.into();
        
        self.file.write_all(&bytes)?;
        Ok(())
    }
}

//type LoggerLock<'a> = OptionMutexGuard<'a, LoadedLogger>;

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
impl MutexProvider for Logger {
    type Data = Option<LoadedLogger>;

    fn access_raw(&self) -> ProtectedAccess<'_, Arc<Mutex<Self::Data>>> {
        ProtectedAccess::new(&self.data)
    }
}
impl OptionMutexProvider<LoadedLogger> for Logger { }
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

    pub fn level(&self) -> Option<LoggerLevel> {
        let data = self.data.lock().unwrap();
        data.as_ref().map(|x| x.level())
    }
}

lazy_static! {
    pub static ref LOG: Logger = Logger::default();
}

pub fn log_global(level: LoggerLevel, contents: String) {
    if !LOG.is_open() {
        return;
    }

    let mut lock = LOG.access();
    match lock.access_mut() {
        Some(v) => log_direct(v, level, contents),
        None => return
    }
}
pub fn log_direct(logger: &mut LoadedLogger, level: LoggerLevel, contents: String) {
    if level >= logger.level() {
        if let Err(e) = logger.write_direct(contents, level) {
            eprintln!("unable to end log because of '{:?}'. Log will be closed", e);
            LOG.reset();
        }
    }
}

#[macro_export(local_inner_macros)]
macro_rules! collapse_level {
    ($level: expr) => {
        {
            #[allow(unreachable_patterns)]
            let true_level: $crate::log::LoggerLevel = match $level {
                $crate::log::LoggerLevel::Debug => $crate::log::LoggerLevel::Debug,
                $crate::log::LoggerLevel::Info => $crate::log::LoggerLevel::Info,
                $crate::log::LoggerLevel::Warning => $crate::log::LoggerLevel::Warning,
                $crate::log::LoggerLevel::Error => $crate::log::LoggerLevel::Error,
                $crate::log::LoggerLevel::Critical => $crate::log::LoggerLevel::Critical,
                //_ => compile_error!("the type passed into this enum must be of LoggerLevel")
            };

            true_level
        }
    }
}

/// A macro that allows for shorthand with logger writting. The callee must sepecify the level as `LoggerLevel`, and the message.
/// Note that this macro will report errors, as they happen. However, if the logger is not open (`logger.is_open() == false`), it will do nothing. 
/// This macro will evaluate the arguments *before* aquiring the lock to the logger. This is to prevent deadlocks, where an argument calls something with the logger.
#[macro_export]
macro_rules! logger_write {
    ($level: expr, $($arg:tt)*) => {
        {
            let contents: String = format!($($arg)*);
            let level = $crate::collapse_level!($level);
            
            
            $crate::log::log_global(level, contents);
        }
    };
    ($log: expr, $level: expr, $($arg:tt)*) => {
        {
            let contents: String = format!($($arg)*);
            let level = $crate::collapse_level!($level);

            $crate::log::log_direct($log, level, contents);
        }
    };
}
/// Writes to the logger with `LoggerLevel::Debug`. Equivalent to `logger_write!(LoggerLevel::Debug, _)`
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Debug, $($arg)*)
        }
    };
    ($log: expr, $($arg:tt)*) => {
        {
            $crate::logger_write!($log, $crate::log::LoggerLevel::Debug, $($arg)*)
        }
    }
}
/// Writes to the logger with `LoggerLevel::Info`. Equivalent to `logger_write!(LoggerLevel::Info, _)`
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Info, $($arg)*)
        }
    };
    ($log: expr, $($arg:tt)*) => {
        {
            $crate::logger_write!($log, $crate::log::LoggerLevel::Info, $($arg)*)
        }
    }
}
/// Writes to the logger with `LoggerLevel::Warning`. Equivalent to `logger_write!(LoggerLevel::Warning, _)`
#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Warning, $($arg)*)
        }
    };
    ($log: expr, $($arg:tt)*) => {
        {
            $crate::logger_write!($log, $crate::log::LoggerLevel::Warning, $($arg)*)
        }
    }
}
/// Writes to the logger with `LoggerLevel::Error`. Equivalent to `logger_write!(LoggerLevel::Error, _)`
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Error, $($arg)*)
        }
    };
    ($log: expr, $($arg:tt)*) => {
        {
            $crate::logger_write!($log, $crate::log::LoggerLevel::Error, $($arg)*)
        }
    }
}
/// Writes to the logger with `LoggerLevel::Critical`. Equivalent to `logger_write!(LoggerLevel::Critical, _)`
#[macro_export]
macro_rules! log_critical {
    ($($arg:tt)*) => {
        {
            use $crate::logger_write;
            logger_write!($crate::log::LoggerLevel::Critical, $($arg)*)
        }
    };
    ($log: expr, $($arg:tt)*) => {
        {
            $crate::logger_write!($log, $crate::log::LoggerLevel::Critical, $($arg)*)
        }
    }
}

#[test]
fn test_logger_write() {
    if let Err(e) = LOG.open("tmp.log", LoggerLevel::Debug, LoggerRedirect::default()) {
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

    LOG.reset();
    assert!(!LOG.is_open());
}