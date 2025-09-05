use std::net::{IpAddr, TcpStream};
use std::process::ExitCode;

use std::io::{stdin, stdout, Stdin, Stdout, Write};
use std::str::FromStr;

use exdisj::{
    log_error, log_debug, log_critical,
    error::FormattingError,
    io::msg::{decode_response, send_request},
    io::log::Logger,
    io::lock::OptionRwProvider
};
use common::msg::{RequestMessages, ResponseMessages};

use crate::config::{KnownHost, CONFIG};
use crate::err::{AVOID_ERR_EXIT, CONFIG_ERR_EXIT, IO_ERR_EXIT};
use crate::tool::connect as tool_connect;

pub fn flush(handle: &mut Stdout) {
    if let Err(e) = handle.flush() {
        panic!("Unable to flush standard output '{e}'");
    }
}
pub fn s_print(message: &str, handle: &mut Stdout) {
    print!("{}", message);
    flush(handle)
}
pub fn prompt(message: &str, out: &mut Stdout, stdin: &mut Stdin) -> String {
    s_print(message, out);

    let mut result = String::new();
    if let Err(e) = stdin.read_line(&mut result) {
        panic!("Unable to read from standard input '{e}'");
    }

    result.trim().to_string()
}

pub enum Commands {
    Quit,
    Status,
    Metrics { amount: usize },
    Help
}
impl FromStr for Commands {
    type Err = FormattingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.trim().to_lowercase();

        if lower == "quit" || lower == "exit" || lower == "close" {
            Ok(Self::Quit)
        }
        else if lower == "status" {
            Ok(Self::Status)
        }
        else if let Some(n) = lower.strip_prefix("metrics") {
            let trimmed = n.trim();
            let number: usize = match trimmed.parse() {
                Ok(v) => v,
                Err(_) => return Err(FormattingError::new(&trimmed, "could not be parsed as a number"))
            };

            Ok(Self::Metrics { amount: number })
        }
        else if lower == "h" || lower == "help" {
            Ok(Self::Help)
        }
        else {
            Err(FormattingError::new(&lower, "could not be constructed into a valid command"))
        }
    }
}

pub fn parse_bool(contents: &str, default: bool) -> bool {
    if contents == "yes" || contents == "true" || contents == "y" {
        true
    }
    else if contents == "no" || contents == "false" || contents == "n" {
        false
    }
    else {
        println!("Unexpected argument '{contents}', assuming '{}'", if default { "yes" } else { "no" });
        default
    }
}

pub fn connect(stdin: &mut Stdin, stdout: &mut Stdout, logger: &Logger) -> Result<TcpStream, ExitCode> {
    let raw = prompt("Has this host been connected to before? ", stdout, stdin).to_lowercase();
    let known = parse_bool(&raw, false);

    let host: IpAddr;
    if known {
        log_debug!(logger, "Getting known hosts");
        let lock = CONFIG.access();

        let hosts = match lock.access() {
            Some(v) => &v.hosts,
            None => {
                log_error!(logger, "Unable to access configuration.");
                return Err(ExitCode::from(CONFIG_ERR_EXIT));
            }
        };

        loop {
            println!("Please choose a host to connect to:");
            for (i, host) in hosts.iter().enumerate() {
                println!("({}) - {}", i + 1, host);
            }
            println!();

            let raw_index = prompt("Index: ", stdout, stdin).to_lowercase();
            if raw_index == "q" {
                log_error!(logger, "Aborting connection process, exiting.");
                return Err(ExitCode::from(AVOID_ERR_EXIT));
            }

            let index: usize = match raw_index.parse() {
                Ok(v) => v,
                Err(e) => {
                    log_debug!(logger, "Unable to parse host choice because of '{e}'.");

                    println!("Unable to parse the value. Try again, or type 'q' to abort.");
                    continue;
                }
            };

            if index == 0 || index >  hosts.len() {
                println!("The option picked is out of range. Try again, or type 'q' to abort.");
                continue;
            }

            host = *hosts[index - 1].addr();
            break;
        }
    }
    else {
        loop {
            let raw_addr = prompt("Please enter an IP address to connect to: ", stdout, stdin);
            if raw_addr == "q" || raw_addr == "Q" {
                log_error!(logger, "Aborting connection process, exiting.");
                    return Err(ExitCode::from(AVOID_ERR_EXIT));
            }

            host = match raw_addr.parse() {
                Ok(v) => v,
                Err(e) => {
                    log_debug!(logger, "unable to parse host becuase of '{e}'");
                    println!("Unable to parse the value. Try again, or type 'q' to abort.");
                    continue;
                }
            };

            break;
        }

        let raw_adding = prompt("Add host to the known hosts? ", stdout, stdin).to_lowercase();
        let adding = parse_bool(&raw_adding, false);
        if adding {
            log_debug!(logger, "Attempting to insert into known hosts.");
            let host_name = prompt("Please enter the host's name: ", stdout, stdin);
            
            let to_insert = KnownHost::new(host_name, host);
            let mut lock = CONFIG.access_mut();
            match lock.access() {
                Some(v) => v.hosts.push(to_insert),
                None => {
                    log_error!(logger, "Unable to access configuration for writing.");
                    let raw_continue = prompt("Unable to write to configuration. Do you wish to continue program execution? ", stdout, stdin).to_lowercase();
                    let cont = parse_bool(&raw_continue, false);

                    if !cont {
                        return Err(ExitCode::from(IO_ERR_EXIT));
                    }
                }
            }
        }
    }

    match tool_connect(host, logger) {
        Ok(v) => Ok(v),
        Err(e) => {
            log_critical!(logger, "Unable to connect: '{e}'");
            Err(ExitCode::from(IO_ERR_EXIT))
        }
    }
}

pub fn cli_entry(logger: &Logger) -> Result<(), ExitCode> {    
    let mut stdin = stdin();
    let mut stdout = stdout();

    println!("Welcome to regis!");
    println!("  Version 0.1.0  ");
    println!("-----------------");

    println!("Please connect to a host.");

    let mut connection = connect(&mut stdin, &mut stdout, logger)?;

    println!("\n Type h or help for help, otherwise type commands.\n");

    loop {
        let raw_message = prompt("> ", &mut stdout, &mut stdin);
        let command = match Commands::from_str(&raw_message) {
            Ok(c) => c,
            Err(e) => {
                log_debug!(logger, "Unable to parse '{e:?}'");
                println!("Unable to parse command (Reason: '{e:?}')");
                continue;
            }
        };

        let message: RequestMessages = match command {
            Commands::Quit => {
                return Ok(())
            }
            Commands::Help => {
                println!("quit|exit|close -> Quits the program");
                println!("metrics AMOUNT -> Requests a specific number of collected metrics from the server.");
                println!("status -> Requests the current status from the server.");
                continue;
            }
            Commands::Metrics { amount } => {
                RequestMessages::Metrics(amount)
            }
            Commands::Status => {
                RequestMessages::Status
            }
        };

        if let Err(e) = send_request(message, &mut connection) {
            log_error!(logger, "Unable to send request to server '{e}'");
            return Err(ExitCode::from(IO_ERR_EXIT));
        }

        let response: ResponseMessages = match decode_response(&mut connection) {
            Ok(v) => v,
            Err(e) => {
                log_error!(logger, "Unable to decode message from server '{e}'");
                return Err(ExitCode::from(IO_ERR_EXIT));
            }
        };

        match response {
            ResponseMessages::Ack(_) => (),
            ResponseMessages::Metrics(m) => {
                println!("Current metrics:\n{m}");
            }
            ResponseMessages::Status(s) => {
                println!("Current status: {s}");
            }
        }
    }
}