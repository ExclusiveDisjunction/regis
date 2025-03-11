pub mod config;
pub mod connect;
pub mod locations;
pub mod message;
pub mod metric;
pub mod orchestra;

use common::log::{LoggerLevel, LoggerRedirect, logging};
use common::{log_debug, log_info, log_warning};
use config::CONFIG;
use orchestra::Orchestrator;

use std::process::ExitCode;

#[tokio::main]
async fn main() -> Result<(), ExitCode> {
    // Start logger

    let level: LoggerLevel;
    let redirect: LoggerRedirect;
    if cfg!(debug_assertions) {
        level = LoggerLevel::Debug;
        redirect = LoggerRedirect::new(Some(LoggerLevel::Debug), true);
    } else {
        level = LoggerLevel::Info;
        redirect = LoggerRedirect::default();
    }

    if let Err(e) = logging.open("run.log", level, redirect) {
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

    orch.run().await
}
