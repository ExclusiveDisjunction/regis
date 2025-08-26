use std::process::ExitCode;
use std::io::Error as IOError;

use exdisj::{log_critical, log_debug, log_error, log_info, log_warning};
use exdisj::io::log::ChanneledLogger;
use tokio::{io::Stdin, runtime::{Runtime, RuntimeMetrics}};

use crate::core::backend::Backend;
use crate::core::conn::ConnectionError;
use crate::core::REGISC_VERSION;

pub fn cli_entry(logger: ChanneledLogger, backend: ChanneledLogger) -> Result<(), ExitCode> {
    // The CLI runs entirely in Tokio, so we need to create a runtime and run the entry.

    let their_logger = logger.clone();

    log_debug!(&logger, "Starting up tokio runtime");
    let runtime = match Runtime::new() {
        Ok(v) => v,
        Err(e) => {
            log_critical!(&logger, "Unable to start tokio runtime, so regisc cannot run. Error: {e:?}");
            return Err( ExitCode::FAILURE )
        }
    };
    let result = runtime.block_on(async move {
        async_cli_entry(their_logger, backend).await
    });

    match result {
        Ok(_) => {
            log_info!(&logger, "Regisc completed with no issues.");
            Ok( () )
        }
        Err(e) => {
            log_error!(&logger, "Regisc completed with the error {e:?}");
            Err( ExitCode::FAILURE )
        }
    }
}

#[derive(Debug)]
pub enum MainLoopFailure {
    Conn(ConnectionError),
    IO(IOError)
}
impl From<IOError> for MainLoopFailure {
    fn from(value: IOError) -> Self {
        Self::IO(value)
    }
}
impl From<ConnectionError> for MainLoopFailure {
    fn from(value: ConnectionError) -> Self {
        Self::Conn(value)
    }
}

pub async fn async_cli_entry(logger: ChanneledLogger, backend: ChanneledLogger) -> Result<(), MainLoopFailure> {
    println!("Regis Console v{REGISC_VERSION}");

    log_info!(&logger, "Starting up backend");
    let mut backend = Backend::spawn(backend).await.map_err(MainLoopFailure::from)?;

    log_info!(&logger, "Backend initialized.");

    log_info!(&logger, "Tasks complete, shutting down backend.");
    if let Err(e) = backend.shutdown(true).await {
        log_warning!(&logger, "The backend shutdown with error '{e:?}'");
    }

    Ok( () )
}