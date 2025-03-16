pub mod config;
pub mod connect;
pub mod loc;
pub mod msg;
pub mod metric;
pub mod orchestra;

use common::log::{LOG, LoggerLevel, LoggerRedirect};
use common::{log_critical, log_debug, log_info, log_warning};
use config::CONFIG;
use daemonize::Daemonize;
use loc::{DAEMON_LOG_DIR, TOTAL_DIR};
use orchestra::Orchestrator;
use regisd_com::loc::{CONSOLE_LOG_DIR, DAEMON_DIR, PID_PATH, STD_ERR_PATH, STD_OUT_PATH};
use tokio::runtime::Runtime;

use std::process::ExitCode;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;

use clap::Parser;

use std::fs::create_dir_all;

#[derive(Parser, Debug)]
struct Options {
    /// Tells the process set logger level to info, and output everything to stdout/stderr.
    #[arg(short, long)]
    verbose: bool,

    /// Tells the process set logger level to debug, and output everything to stdout/stderr.
    #[arg(long)]
    debug: bool,

    /// Instructs the process to run as a daemon.
    #[arg(short, long)]
    daemon: bool,

    /// The location that standard out should go to. Ignored if not a daemon.
    #[arg(long, value_name = "FILE")]
    stdout: Option<String>,

    /// The location that standard error should go to. Ignored if not a daemon.
    #[arg(long, value_name = "FILE")]
    stderr: Option<String>
}

async fn start_orch() -> Result<(), ExitCode> {
    log_info!("Init complete, handling tasks to orchestrator");
    let orch = Orchestrator::initialize();

    let result = orch.run().await;
    CONFIG.save(loc::DAEMON_CONFIG_PATH).map_err(|_| ExitCode::FAILURE)?;
    
    result
}

fn begin_runtime() -> Result<(), ExitCode> {
    let rt = match Runtime::new() {
        Ok(v) => v,
        Err(e) => {
            log_critical!("Unable to start tokio runtime '{e}'");
            return Err(ExitCode::FAILURE)
        }
    };

    rt.block_on( async {
        start_orch().await
    })
}

fn main() -> Result<(), ExitCode> {
    let cli = Options::parse();

    let level: LoggerLevel;
    let redirect: LoggerRedirect;
    if cfg!(debug_assertions) || cli.debug {
        level = LoggerLevel::Debug;
        redirect = LoggerRedirect::new(Some(LoggerLevel::Debug), true);
    }
    else if cli.verbose {
        level = LoggerLevel::Info;
        redirect = LoggerRedirect::new(Some(LoggerLevel::Info), true);
    }
    else {
        level = LoggerLevel::Info;
        redirect = LoggerRedirect::new(Some(LoggerLevel::Warning), true);
    }

    let today = chrono::Local::now();
    if let Err(e) = create_dir_all(TOTAL_DIR) {
        eprintln!("Unable to startup service. Checking of directory structure failed '{e}'.");
        return Err(ExitCode::FAILURE);
    }

    if let Err(e) = create_dir_all(DAEMON_LOG_DIR) {
        eprintln!("Unable to startup service. Checking of directory structure failed '{e}'.");
        return Err(ExitCode::FAILURE);
    }
    if let Err(e) = create_dir_all(CONSOLE_LOG_DIR) {
        eprintln!("Unable to startup service. Checking of directory structure failed '{e}'.");
        return Err(ExitCode::FAILURE);
    }

    if let Err(e) = fs::set_permissions(DAEMON_LOG_DIR, fs::Permissions::from_mode(0o777)) {
        eprintln!("Unable to make logs open to everyone. '{e}'");
        return Err(ExitCode::FAILURE);
    }
    if let Err(e) = fs::set_permissions(CONSOLE_LOG_DIR, fs::Permissions::from_mode(0o777)) {
        eprintln!("Unable to make logs open to everyone. '{e}'");
        return Err(ExitCode::FAILURE);
    }

    if let Err(e) = create_dir_all(DAEMON_DIR) {
        eprintln!("Unable to startup service. Checking of directory structure failed '{e}'.");
        return Err(ExitCode::FAILURE);
    }

    let logger_path = format!("{}{:?}-run.log", DAEMON_LOG_DIR, today);

    if let Err(e) = LOG.open(logger_path, level, redirect) {
        eprintln!("Unable to start logger because '{e}'");
        return Err(ExitCode::FAILURE);
    }

    log_info!("Launching regisd...");

    log_debug!("Loading configuration");
    if let Err(e) = CONFIG.open(loc::DAEMON_CONFIG_PATH) {
        log_warning!(
            "Unable to load configuration, creating default for this initalization. Error: '{:?}'",
            e
        );
        CONFIG.set_to_default();
    }
    log_info!("Configuration loaded.");

    if cli.daemon {
        let stdout_path = if let Some(p) = cli.stdout.as_deref() {
            p
        }
        else {
            STD_OUT_PATH
        };

        let stderr_path = if let Some(p) = cli.stderr.as_deref() {
            p
        }
        else {
            STD_ERR_PATH
        };

        let stdout = match File::create(stdout_path) {
            Ok(f) => f,
            Err(e) => {
                log_critical!("Unable to open stdout file at path '{stdout_path}' because of '{e}'");
                return Err(ExitCode::FAILURE);
            }
        };
        let stderr = match File::create(stderr_path) {
            Ok(f) => f,
            Err(e) => {
                log_critical!("Unable to open stderr file at path '{stderr_path}' because of '{e}'");
                return Err(ExitCode::FAILURE);
            }
        };

        log_info!("Starting regisd as a daemon...");

        let daemonize = Daemonize::new()
            .pid_file(PID_PATH)
            .stdout(stdout)
            .stderr(stderr)
            .chown_pid_file(true)
            .working_directory("/");

        match daemonize.start() {
            Ok(_) => {
                log_info!("Daemon loaded. Running process.");
                let result = begin_runtime();
                log_info!("Daemon finished.");

                result
            }
            Err(e) => {
                log_critical!("Unable to start daemon '{e}.");
                Err(ExitCode::FAILURE)
            }
        }
    }
    else {
        begin_runtime()
    }
}
