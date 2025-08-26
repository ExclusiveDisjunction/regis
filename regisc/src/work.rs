use exdisj::io::log::{ConsoleColor, Logger, LoggerLevel, LoggerRedirect, Prefix};
use exdisj::version::Version;
use exdisj::{log_critical, log_info};
use exdisj::io::msg::send_request_async;

use common::loc::{CONSOLE_LOG_DIR, COMM_PATH};
use common::msg::{ConsoleRequests, ConsoleResponses};

use tokio::net::UnixStream;
use tokio::fs::create_dir_all;
use clap::{Parser, ValueEnum};

use std::process::ExitCode;

pub const REGISC_VERSION: Version = Version::new(0, 2, 0);

#[derive(ValueEnum, Debug, Clone, Copy)]
enum QuickCommand {
    /// Instruct the daemon to gracefully shutdown
    Shutdown,
    /// Instruct the daemon to reload its configuration file
    Config,
    /// Determines if the daemon is running.
    Poll,
}

#[derive(Parser, Debug)]
#[command(name = "regisc", version = "0.2.0", about = "An interface to communicate with the regisd process, if it is running.")]
struct Options {
    /// Connects to regisd, sends the specified message, and closes the connection. Cannot be combined with --gui.
    #[arg(short, long)]
    quick: Option<QuickCommand>,

    /// When used, regisc will output more log messages. The default is false, and the default level will be warning.
    #[arg(short, long)]
    verbose: bool,

    /// When used, regisc will open as a graphical user interface. Cannot be combined with --quick.
    #[cfg(feature="gui")]
    #[arg(long)]
    gui: bool
}

pub async fn entry() -> Result<(), ExitCode> {
    // Parse command
    let command = Options::parse();

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
        return Err( ExitCode::FAILURE );
    }

    let today = chrono::Local::now();
    let logger_path = format!("{}{:?}-run.log", CONSOLE_LOG_DIR, today);

    let logger = match Logger::new(logger_path, level, redirect)  {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error! Unable to start log (error: '{e}'). Exiting.");
            return Err( ExitCode::FAILURE );
        }
    };

    if let Some(q) = command.quick {
        log_info!(&logger, "Sending quick command {q:?}");

        

        return Ok( () ); 
    }

    log_info!(&logger, "Starting runtime");
    let runtime_channel = logger.make_channel(Prefix::new_const("Runtime", ConsoleColor::Green));
    let end_channel = logger.make_channel(Prefix::new_const("User", ConsoleColor::Cyan));

    /*
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
     */

    Ok( () )
}