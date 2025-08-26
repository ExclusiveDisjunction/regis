use exdisj::{
    log_critical, log_info,
    io::log::{
        Logger,
        LoggerLevel,
        LoggerRedirect
    }
};

use common::loc::{CONSOLE_LOG_DIR, DAEMON_DIR, PID_PATH, STD_ERR_PATH, STD_OUT_PATH, DAEMON_LOG_DIR, TOTAL_DIR, DAEMON_CONFIG_PATH};

use crate::orchestra::Orchestrator;
use crate::config::CONFIG;
use crate::failure::DaemonFailure;

use tokio::runtime::Runtime;
use daemonize::Daemonize;

use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;

use clap::Parser;

use std::fs::create_dir_all;

#[derive(Parser, Debug)]
pub struct Options {
    /// Tells the process set logger level to info, and output everything to stdout/stderr.
    #[arg(short, long)]
    pub verbose: bool,

    /// Tells the process set logger level to debug, and output everything to stdout/stderr.
    #[arg(long)]
    pub debug: bool,

    /// Instructs the process to run as a daemon.
    #[arg(short, long)]
    pub daemon: bool,

    /// Instructs the process to not use authentication and encryption over the TCP channel.
    #[arg(short, long)]
    pub no_authentiation: bool,

    /// When this is true, if the configuration is invalid, it will reset the configuration and continue on. Otherwise, the program will exit.
    #[arg(long)]
    pub override_config: bool,

    /// The location that standard out should go to. Ignored if not a daemon.
    #[arg(long, value_name = "FILE")]
    pub stdout: Option<String>,

    /// The location that standard error should go to. Ignored if not a daemon.
    #[arg(long, value_name = "FILE")]
    pub stderr: Option<String>
}

pub async fn start_orch(log: &Logger, options: Options) -> Result<(), DaemonFailure> {
    log_info!(log, "Init complete, handling tasks to orchestrator");
    let orch = Orchestrator::initialize(log, options);

    let result = orch.run().await;
    CONFIG.save(DAEMON_CONFIG_PATH).map_err(|_| DaemonFailure::IOError)?;
    
    result
}

pub fn begin_runtime(log: &Logger, options: Options) -> Result<(), DaemonFailure> {
    let rt = match Runtime::new() {
        Ok(v) => v,
        Err(e) => {
            log_critical!(log, "Unable to start tokio runtime '{e}'");
            return Err( DaemonFailure::RuntimeFailure );
        }
    };

    rt.block_on( async {
        start_orch(log, options).await
    })
}

pub fn start_logger(options: &Options) -> std::io::Result<Logger> {
    let level: LoggerLevel;
    let redirect: LoggerRedirect;
    if cfg!(debug_assertions) || options.debug {
        level = LoggerLevel::Debug;
        redirect = LoggerRedirect::new(Some(LoggerLevel::Debug), true);
    }
    else if options.verbose {
        level = LoggerLevel::Info;
        redirect = LoggerRedirect::new(Some(LoggerLevel::Info), true);
    }
    else {
        level = LoggerLevel::Info;
        redirect = LoggerRedirect::new(Some(LoggerLevel::Warning), true);
    }

    let today = chrono::Local::now();
    let logger_path = format!("{DAEMON_LOG_DIR}{today:?}-run.log");

    Logger::new(logger_path, level, redirect)
}

pub fn create_paths() -> std::io::Result<()> {
    create_dir_all(TOTAL_DIR)?;
    create_dir_all(DAEMON_LOG_DIR)?;
    create_dir_all(CONSOLE_LOG_DIR)?;
    create_dir_all(DAEMON_DIR)?;

    fs::set_permissions(DAEMON_LOG_DIR, fs::Permissions::from_mode(0o777))?;
    fs::set_permissions(CONSOLE_LOG_DIR, fs::Permissions::from_mode(0o777))?;

    Ok( () )
}

pub fn run_as_daemon(log: &Logger, options: Options) -> Result<(), DaemonFailure> {
    let stdout_path = options.stdout.as_deref().unwrap_or(STD_OUT_PATH);
    let stderr_path = options.stderr.as_deref().unwrap_or(STD_ERR_PATH);

    let constructor = || -> Result<(File, File), std::io::Error> {
        Ok( ( File::create(stdout_path)?, File::create(stderr_path)? ) )
    };

    let (stdout, stderr) = match constructor() {
        Ok(v) => v,
        Err(e) => {
            log_critical!(log, "Unable to construct the stdout and/or stderr files at '{}' and '{}', respectivley. Reason: '{e}'", stdout_path, stderr_path);
            return Err( DaemonFailure::SetupStreamError );
        }
    };

    log_info!(log, "Starting regisd as a daemon...");

    let daemonize = Daemonize::new()
        .pid_file(PID_PATH)
        .stdout(stdout)
        .stderr(stderr)
        .chown_pid_file(true)
        .working_directory("/");

    match daemonize.start() {
        Ok(_) => {
            log_info!(log, "Daemon loaded. Running process.");
            let result = begin_runtime(log, options);
            log_info!(log, "Daemon finished.");

            result
        }
        Err(e) => {
            log_critical!(log, "Unable to start daemon '{e}.");
            Err( DaemonFailure::DaemonizeFailure )
        }
    }
}