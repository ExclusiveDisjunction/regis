pub mod loc;
pub mod config;
pub mod gui;
pub mod cli;
pub mod err;
pub mod tool;

use clap::Parser;
use cli::cli_entry;

use exdisj::{
    log_error, log_info,
    io::log::{
        LoggerLevel, LoggerRedirect, Logger
    },
    io::lock::OptionRwProvider
};

use config::CONFIG;
use err::{CHECK_ERR_EXIT, LOG_ERR_EXIT};
//use gui::gui_entry;
use loc::{get_client_dir, get_config_path, get_log_dir};

use std::fs::create_dir_all;
use std::process::{ExitCode, exit};

#[derive(Parser, Debug)]
struct Options {
    /// Instructs the program to run in CLI mode, and to not load a GUI.
    #[arg(short = 'g', long)]
    graphical: bool,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    debug: bool
}

fn ensure_directories() {
    if let Err(e) = create_dir_all(get_client_dir()) {
        eprintln!("Unable to ensure directory structure. '{e}'");
        exit(CHECK_ERR_EXIT as i32);
    }

    if let Err(e) = create_dir_all(get_log_dir()) {
        eprintln!("Unable to ensure directory structure. '{e}'");
        exit(CHECK_ERR_EXIT as i32);
    }
}

fn main() -> Result<(), ExitCode> {
    let command = Options::parse();

    /*
        Level:
            Debug || command.debug => LoggerLevel::Debug,
            _ => LoggerLevel::Info

        Redirect: 
            Debug || command.debug => Some(LoggerLevel::Debug), true
            command.verbose => Some(LoggerLevel::Info), true
            _ => None, true
     */

    let is_debugging = cfg!(debug_assertions);
    let level = if is_debugging || command.debug {
        LoggerLevel::Debug
    }
    else {
        LoggerLevel::Info
    };

    let redirect = if is_debugging || command.debug {
        LoggerRedirect::new(Some(LoggerLevel::Debug), true)
    }
    else if command.verbose {
        LoggerRedirect::new(Some(LoggerLevel::Info), true)
    }
    else {
        LoggerRedirect::new(None, true) 
    };

    ensure_directories();

    let today = chrono::Local::now();
    let logger_path = get_log_dir().join(today.to_string());

    let logger = match Logger::new(logger_path, level, redirect) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Unable to start log '{e:?}'");
            return Err(LOG_ERR_EXIT.into());
        }
    };

    if let Err(e) = CONFIG.open(get_config_path()) {
        eprintln!("Unable to load configuration '{e:?}'. Reseting to default.");
        CONFIG.set_to_default();
    }

    log_info!(&logger, "Starting regis service.");
    let result = if command.graphical {
        log_info!(&logger, "GUI mode activated.");
        crate::gui::gui_entry();
        Ok( () )
    }
    else {
        log_info!(&logger, "CLI mode activated.");
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(v) => v,
            Err(e) => {
                log_error!(&logger, "Unable to startup tokio runtime '{e:?}'.");
                return Err( ExitCode::FAILURE );
            }
        };

        runtime.block_on(cli_entry(&logger))
    };

    log_info!(&logger, "Saving configuration...");
    if let Err(e) = CONFIG.save(get_config_path()) {
        log_error!(&logger, "Unable to save configuration '{e:?}'.");
    }
    else {
        log_info!(&logger, "Configuration saved.");
    }

    result
}
