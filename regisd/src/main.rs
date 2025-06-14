pub mod config;
pub mod connect;
pub mod msg;
pub mod metric;
pub mod orchestra;
pub mod failure;
pub mod setup;
pub mod sess;

use exdisj::{log_critical, log_debug, log_info, log_warning};
use exdisj::lock::OptionRwProvider;
use common::loc::DAEMON_CONFIG_PATH;

use config::CONFIG;
use failure::DaemonFailure;

use std::panic::catch_unwind;
use std::process::ExitCode;

use clap::Parser;

fn run() -> Result<(), DaemonFailure> {
    let cli = setup::Options::parse();
    if let Err(e) = setup::create_paths() {
        eprintln!("Unable to create paths. '{e}'");
        return Err( DaemonFailure::SetupDirectoryError );
    }

    if !setup::start_logger(&cli) {
        eprintln!("Unable to start logger.");
        return Err( DaemonFailure::LoggerError );
    }

    log_info!("Launching regisd...");

    log_debug!("Loading configuration");
    if let Err(e) = CONFIG.open(DAEMON_CONFIG_PATH) {
        if cli.override_config {
            log_warning!(
                "Unable to load configuration, creating default for this initalization. Error: '{:?}'",
                e
            );
            CONFIG.set_to_default();
        }
        else {
            log_critical!("The configuration was invalid, reason '{:?}'\n. Since the configuration could not be defaulted, the program will exit.\nTo reset the configuration, run the command with --override_config.", e);
            return Err( DaemonFailure::ConfigurationError );
        }
    }
    log_info!("Configuration loaded.");

    let result = catch_unwind(|| {
        if cli.daemon {
            setup::run_as_daemon(cli)
        }
        else {
            setup::begin_runtime(cli)
        }
    });

    match result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Note, unexpected error occured, causing a panic. '{e:?}'");
            Err( DaemonFailure::UnexepctedError )
        }
    }
}

fn main() -> Result<(), ExitCode> {
    let result = run();

    if let Err(e) = result {
        eprintln!("Notice: Software exited with code {}, description: '{}'", e as u8, e);
        Err( e.into() )
    }
    else {
        Ok( () )
    }
}
