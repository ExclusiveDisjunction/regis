use common::log::{logging, LoggerLevel, LoggerRedirect};
use common::version::Version;
use common::{log_critical, log_info};
use common::msg::{decode_response, send_request, Acknoledgement, DecodeError};
use regisd_com::msg::{AuthenticateRequest, ShutdownRequest, UpdateConfigRequest};

#[cfg(all(unix))]
use regisd_com::loc::SERVER_COMM_PATH;

#[cfg(all(unix))]
use tokio::net::UnixStream;
use clap::{Parser, Subcommand};

use std::process::exit;

pub const REGISC_VERSION: Version = Version::new(0, 1, 0);

#[derive(Parser, Debug)]
#[command(name = "regisc", version = "0.1.0", about = "An interface to communicate with the regisd process, if it is running.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long)]
    verbose: bool
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Allow any authentication requests that the server currently has
    Auth,
    /// Instruct the daemon to gracefully shutdown
    Shutdown,
    /// Tell the daemon to reload its configuration file
    Config
}

#[cfg(all(unix))]
async fn connect() -> Result<UnixStream, std::io::Error> {
    let mut stream = UnixStream::connect(SERVER_COMM_PATH).await?;
    let ack: Acknoledgement = decode_response(&mut stream).await
        .map_err(|e| {
            match e {
                DecodeError::IO(e) => e,
                DecodeError::Serde(e) => std::io::Error::new(std::io::ErrorKind::NetworkUnreachable, e),
                DecodeError::UTF(e) => std::io::Error::new(std::io::ErrorKind::NetworkUnreachable, e),
            }
        })?;

    if !ack.is_ok() {
        log_critical!("Unable to open connection, with code '{}', message '{}'", ack.code(), ack.message().unwrap_or("(No message)"));
        return Err(std::io::Error::new(std::io::ErrorKind::ResourceBusy, "the network was not able to serve the connection."));
    }
    
    Ok(stream)
}

pub async fn entry() {
    // Parse command
    let command = match Cli::try_parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid command usage. Please type regisc --help for explinations.");
            exit(2);
        }
    };

    // Establish logger
    let level: LoggerLevel;
    let redirect: LoggerRedirect;
    if cfg!(debug_assertions) || command.verbose {
        level = LoggerLevel::Debug;
        redirect = LoggerRedirect::new(Some(LoggerLevel::Debug), true);
    }
    else {
        level = LoggerLevel::Warning;
        redirect = LoggerRedirect::default();
    }

    if logging.open("run.log", level, redirect).is_err() {
        eprintln!("Error! Unable to start log. Exiting.");
        exit(1);     
    }

    //Connect
    log_info!("Connecting to regisd...");
    let mut stream = match connect().await {
        Ok(v) => v,
        Err(e) => {
            log_critical!("Unable to connect to regisd: '{}'. Please ensure that it is loaded & running.", e);
            exit(3);
        }
    };

   let result = match command.command {
        Commands::Auth => {
            log_info!("Sending Authentication message to regisd...");
            send_request(AuthenticateRequest, &mut stream).await
        },
        Commands::Config => {
            log_info!("Sending Config message to regisd...");
            send_request(UpdateConfigRequest, &mut stream).await
        },
        Commands::Shutdown => {
            log_info!("Sending Shutdown message to regisd...");
            send_request(ShutdownRequest, &mut stream).await
        }
    };
    
    if let Err(e) = result {
        log_critical!("Unable to send message, reason '{e}'.");
        exit(1);
    }
    else {
        log_info!("Regisc complete. Message sent.");
    }
}