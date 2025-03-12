pub mod config;
pub mod connect;
pub mod locations;
pub mod message;
pub mod metric;
pub mod orchestra;

use common::log::{logging, LoggerLevel, LoggerRedirect};
use common::{log_debug, log_info, log_warning};
use config::CONFIG;
use locations::{LOG_DIR, TOTAL_DIR};
use orchestra::Orchestrator;

use std::process::ExitCode;

use clap::Parser;

use tokio::fs::create_dir_all;

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

#[tokio::main]
async fn main() -> Result<(), ExitCode> {
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
    if let Err(e) = create_dir_all(TOTAL_DIR).await {
        eprintln!("Unable startup service. Checking of directory structure failed '{e}'.");
        return Err(ExitCode::FAILURE);
    }

    if let Err(e) = create_dir_all(LOG_DIR).await {
        eprintln!("Unable startup service. Checking of directory structure failed '{e}'.");
        return Err(ExitCode::FAILURE);
    }

    let logger_path = format!("{}/{:?}-run.log", LOG_DIR, today);

    if let Err(e) = logging.open(logger_path, level, redirect) {
        eprintln!("Unable to start logger because '{e}'");
        return Err(ExitCode::FAILURE);
    }

    log_info!("Launching regisd...");

    log_debug!("Loading configuration");
    if let Err(e) = CONFIG.open(locations::CONFIG_PATH) {
        log_warning!(
            "Unable to load configuration, creating default for this initalization. Error: '{:?}'",
            e
        );
        CONFIG.set_to_default();
    }
    log_info!("Configuration loaded.");

    log_info!("Check complete, handling tasks to orchestrator");
    let orch = Orchestrator::initialize();

    let result = orch.run().await;
    CONFIG.save(locations::CONFIG_PATH).map_err(|_| ExitCode::FAILURE)?;
    
    result
}
