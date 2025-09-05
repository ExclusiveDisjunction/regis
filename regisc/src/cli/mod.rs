use std::process::ExitCode;
use std::io::Error as IOError;

use common::msg::ConsoleAuthRequests;
use exdisj::{log_critical, log_debug, log_error, log_info, log_warning};
use exdisj::io::log::ChanneledLogger;
use tokio::io::{stdout, AsyncWriteExt, Lines, Stdin, Stdout};
use tokio::{
    runtime::Runtime,
    io::{AsyncBufReadExt as _, BufReader, stdin}
};
use clap::Parser;

use crate::core::backend::{Backend, BackendRequests};
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
    IO(IOError),
    DeadBackend
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

pub async fn prompt(stdout: &mut Stdout, lines: &mut Lines<BufReader<Stdin>>) -> Result<String, IOError> {
    stdout.write("> ".as_bytes()).await?;
    stdout.flush().await?;

    let raw_command = lines.next_line()
        .await?
        .unwrap_or("quit".to_string());

    Ok( raw_command )
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ConfigCommands {
    Reload,
    Get,
    Update
}
impl Into<BackendRequests> for ConfigCommands {
    fn into(self) -> BackendRequests {
        match self {
            Self::Reload => BackendRequests::GetConfig,
            Self::Get => BackendRequests::GetConfig,
            Self::Update => BackendRequests::UpdateConfig
        }
    }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum AuthCommands {
    Pending,
    Revoke { id: u64 },
    Approve { id: u64 },
    Users,
    History { id: u64 }
}
impl Into<ConsoleAuthRequests> for AuthCommands {
    fn into(self) -> ConsoleAuthRequests {
        match self {
            Self::Pending => ConsoleAuthRequests::Pending,
            Self::Users => ConsoleAuthRequests::AllUsers,
            Self::History { id } => ConsoleAuthRequests::UserHistory(id),
            Self::Approve { id } => ConsoleAuthRequests::Approve(id),
            Self::Revoke { id } => ConsoleAuthRequests::Revoke(id)
        }
    }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum CliCommands {
    Quit,
    Poll,
    #[command(subcommand)]
    Config(ConfigCommands),
    #[command(subcommand)]
    Auth(AuthCommands)
}

#[derive(Debug, clap::Parser)]
pub struct CliParser {
    #[command(subcommand)]
    command: CliCommands
}

pub async fn async_cli_entry(logger: ChanneledLogger, backend: ChanneledLogger) -> Result<(), MainLoopFailure> {
    println!("Regis Console v{REGISC_VERSION}");

    log_info!(&logger, "Starting up backend");
    let mut backend = Backend::spawn(backend).await.map_err(MainLoopFailure::from)?;

    log_info!(&logger, "Backend initialized.");

    log_info!(&logger, "Begining main loop.");

    let input = stdin();
    let mut stdout = stdout();
    let reader = BufReader::new(input);
    let mut lines = reader.lines();

    println!("Type a command, or type quit.");

    loop {
        let raw_command = prompt(&mut stdout, &mut lines).await.map_err(MainLoopFailure::from)?;
        let trim = raw_command.trim();
        let parts = trim.split_whitespace();
        let parts = [">"].into_iter().chain(parts);
        let command = match CliParser::try_parse_from(parts) {
            Ok(v) => v.command,
            Err(e) => {
                println!("Unable to parse command\n'{e}'");
                continue;
            }
        };

        let request = match command {
            CliCommands::Quit => break,
            CliCommands::Poll => BackendRequests::Poll,
            CliCommands::Config(config) => {
                config.into()
            },
            CliCommands::Auth(auth) => {
                BackendRequests::Auth(
                    auth.into()
                )
            }
        };

        log_info!(&logger, "Sending request '{:?}' to backend", &request);
        match backend.send_with_response(request).await {
            Some(m) => {
                log_info!(&logger, "The backend sent a response: {m:?}.");
            },
            None => {
                log_error!(&logger, "The backend is inactive. Aborting regisc.");
                return Err( MainLoopFailure::DeadBackend );
            }
        };
    }

    log_info!(&logger, "Tasks complete, shutting down backend.");
    if let Err(e) = backend.shutdown(true).await {
        log_warning!(&logger, "The backend shutdown with error '{e:?}'");
    }

    Ok( () )
}