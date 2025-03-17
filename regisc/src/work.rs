use common::log::{LOG, LoggerLevel, LoggerRedirect};
use common::version::Version;
use common::{log_critical, log_info};
use common::msg::send_request_async;
use regisd_com::msg::ConsoleRequests;

use regisd_com::loc::{COMM_PATH, CONSOLE_LOG_DIR};

use tokio::net::UnixStream;
use tokio::fs::create_dir_all;
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
    /// Instruct the daemon to approve authentication requests
    Auth,
    /// Instruct the daemon to gracefully shutdown
    Shutdown,
    /// Instruct the daemon to reload its configuration file
    Config,
    /// Determines if the daemon is running.
    Poll,
}

pub async fn entry() {
    // Parse command
    let command = Cli::parse();

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

    if let Err(e) = create_dir_all(CONSOLE_LOG_DIR).await {
        eprintln!("Unable to startup logs. Checking of directory structure failed '{e}'.");
        exit(1);
    }

    let today = chrono::Local::now();
    let logger_path = format!("{}{:?}-run.log", CONSOLE_LOG_DIR, today);

    if LOG.open(logger_path, level, redirect).is_err() {
        eprintln!("Error! Unable to start log. Exiting.");
        exit(1);     
    }

    // Parse request
    let request = match command.command {
        Commands::Auth => ConsoleRequests::Auth,
        Commands::Config => ConsoleRequests::Config,
        Commands::Shutdown => ConsoleRequests::Shutdown,
        Commands::Poll => ConsoleRequests::Poll,
    };

    //Connect
    log_info!("Connecting to regisd...");
    let mut stream = match UnixStream::connect(COMM_PATH).await {
        Ok(v) => v,
        Err(e) => {
            if request == ConsoleRequests::Poll {
                log_critical!("Daemon is not active, due to failure to connect to it.");
            }
            else {
                log_critical!("Unable to connect to regisd: '{}'. Please ensure that it is loaded & running.", e);
            }
            exit(3);
        }
    };

    // Send message
    let result = send_request_async(request, &mut stream).await;
    
    if let Err(e) = result {
        if request == ConsoleRequests::Poll {
            log_critical!("Daemon is not active, due to failure to send a message to it.");
        }
        else {
            log_critical!("Unable to send message, reason '{e}'.");
        }
        exit(1);
    }
    else {
        if request == ConsoleRequests::Poll {
            println!("The daemon is active.")
        }

        log_info!("Regisc complete. Message sent.");
    }
}