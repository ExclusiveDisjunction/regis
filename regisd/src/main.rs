pub mod config;
pub mod connect;
pub mod msg;
pub mod metric;
pub mod orchestra;
pub mod failure;
pub mod setup;
pub mod sess;

use exdisj::{log_critical, log_info, log_warning};
use exdisj::io::lock::OptionRwProvider;
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

    let logger = match setup::start_logger(&cli) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Unable to crate a logger: '{e}'");
            return Err( DaemonFailure::LoggerError );
        }
    };

    log_info!(&logger, "Launching regisd...");

    log_info!(&logger, "Loading configuration");
    if let Err(e) = CONFIG.open(DAEMON_CONFIG_PATH) {
        if cli.override_config {
            log_warning!(
                &logger,
                "Unable to load configuration, creating default for this initalization. Error: '{:?}'",
                e
            );
            CONFIG.set_to_default();
        }
        else {
            log_critical!(&logger, "The configuration was invalid, reason '{:?}'\n. Since the configuration could not be defaulted, the program will exit.\nTo reset the configuration, run the command with --override-config.", e);
            return Err( DaemonFailure::ConfigurationError );
        }
    }
    log_info!(&logger, "Configuration loaded.");

    let result = catch_unwind(|| {
        if cli.daemon {
            setup::run_as_daemon(&logger, cli)
        }
        else {
            setup::begin_runtime(&logger, cli)
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
