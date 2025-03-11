use common::log::{logging, LoggerLevel, LoggerRedirect};
use common::version::Version;
use common::{log_critical, log_info};
use common::msg::send_request;
use regisd_com::msg::ConsoleRequests;

use regisd_com::loc::SERVER_COMM_PATH;

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
    let mut stream = match UnixStream::connect(SERVER_COMM_PATH).await {
        Ok(v) => v,
        Err(e) => {
            log_critical!("Unable to connect to regisd: '{}'. Please ensure that it is loaded & running.", e);
            exit(3);
        }
    };

   let request = match command.command {
        Commands::Auth => ConsoleRequests::Auth,
        Commands::Config => ConsoleRequests::Config,
        Commands::Shutdown => ConsoleRequests::Shutdown
    };

    let result = send_request(request, &mut stream).await;
    
    if let Err(e) = result {
        log_critical!("Unable to send message, reason '{e}'.");
        exit(1);
    }
    else {
        log_info!("Regisc complete. Message sent.");
    }
}