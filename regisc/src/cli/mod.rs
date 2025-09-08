use std::process::ExitCode;
use std::io::Error as IOError;

use common::config::Configuration;
use common::loc::CLIENTS_PORT;
use common::msg::{ConsoleAuthRequests, PendingUser, UserDetails, UserSummary};
use exdisj::{log_critical, log_debug, log_error, log_info, log_warning};
use exdisj::io::log::{ChanneledLogger, LoggerBase};
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

pub async fn prompt_raw<V>(content: &V, stdout: &mut Stdout, lines: &mut Lines<BufReader<Stdin>>) -> Result<String, IOError> 
    where V: AsRef<[u8]> + ?Sized {
        stdout.write(content.as_ref()).await?;
        stdout.flush().await?;

        let raw_command = lines.next_line()
            .await?
            .unwrap_or("quit".to_string());

        Ok( raw_command )
}
pub async fn prompt(stdout: &mut Stdout, lines: &mut Lines<BufReader<Stdin>>) -> Result<String, IOError> {
    prompt_raw("> ", stdout, lines).await
}

#[derive(Debug, Clone, Copy, clap::Subcommand, PartialEq, Eq)]
pub enum ConfigCommands {
    Reload,
    Get,
    Update
}
impl Into<BackendRequests> for ConfigCommands {
    fn into(self) -> BackendRequests {
        match self {
            Self::Reload => BackendRequests::ReloadConfig,
            Self::Get => BackendRequests::GetConfig,
            Self::Update => todo!("Implement the system that converts this into a series of commands")
        }
    }
}

#[derive(Debug, Clone, Copy, clap::Subcommand)]
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
    Clear,
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

pub fn print_auth_approve_result<L: LoggerBase>(logger: &L, id: u64, message: Option<bool>) {
    match message {
        Some(true) => println!("User with id {id} was approved."),
        Some(false) => println!("User with id {id} could not be approved (Perhaps it has a different id?)."),
        None => log_error!(logger, "Unable to decode the approval status for user with id {id}")
    }
}
pub fn print_revoke_result<L: LoggerBase>(logger: &L, id: u64, message: Option<bool>) {
    match message {
        Some(true) => println!("User with id {id} was revoked."),
        Some(false) => println!("User with id {id} could not be revoked (Perhaps it has a different id?)."),
        None => log_error!(logger, "Unable to decode the revoked status for user with id {id}")
    }
}

pub fn print_with_deserialization<'de, L: LoggerBase, V: serde::Deserialize<'de>, F>(logger: &L, message: &'de [u8], inner: F) where F: FnOnce(V) -> () {
    match serde_json::from_slice::<V>(&message) {
        Ok(value) => inner(value),
        Err(e) => log_error!(logger, "Unable to decode the users (error: '{e:?}'.")
    }
}
pub fn print_user_history_table(user: UserDetails) {
    println!("User history for id {} (Aka '{}'):", user.id(), user.nickname());
    println!("| {:^25} | {:^30} |", "From IP", "Time");
    println!("| {:-^25} | {:-^30} |", "", "");

    for history in user.history() {
        println!("| {:>25} | {:>30} |", history.from_ip(), history.at_time())
    }
}
pub fn print_all_users_table(users: Vec<UserSummary>) {
    if users.is_empty() {
        println!("Regisd has no users.");
    }
    else {
        println!("| {:^7} | {:^20} |", "ID", "Nickname");
        println!("| {:-^7} | {:-^20} |", "", "");
        for user in users {
            println!("| {:^7} | {:^20} |", user.id(), user.nickname());
        }
    }
}
pub fn print_pending_users_table(pending_users: Vec<PendingUser>) {
    if pending_users.is_empty() {
        println!("Regisd has no pending users for authentication.");
    }
    else {
        println!("| {:^7} | {:^25} | {:^30} |", "ID", "From IP", "Time");
        println!("| {:-^7} | {:-^25} | {:-^30} |", "", "", "");
        for pending_user in pending_users {
            println!("| {:^7} | {:>25} | {:>30} |", pending_user.id(), pending_user.ip(), pending_user.time())
        }
    }
}

pub fn print_auth_response<L: LoggerBase>(logger: &L, inner: AuthCommands, message: &[u8]) {
    match inner {
        AuthCommands::Approve { id } => print_auth_approve_result(logger, id, serde_json::from_slice(message).ok()),
        AuthCommands::Revoke { id } => print_revoke_result(logger, id, serde_json::from_slice(message).ok()),
        AuthCommands::Pending => print_with_deserialization(logger, message, print_pending_users_table),
        AuthCommands::Users => print_with_deserialization(logger, message, print_all_users_table),
        AuthCommands::History { id: _ } => print_with_deserialization(logger, message, print_user_history_table),
    }
}

pub async fn prompt_command(stdout: &mut Stdout, lines: &mut Lines<BufReader<Stdin>>) -> Result<Option<CliCommands>, MainLoopFailure> {
    let raw_command = prompt(stdout, lines).await.map_err(MainLoopFailure::from)?;
    let trim = raw_command.trim();
    let parts = trim.split_whitespace();
    let parts = [">"].into_iter().chain(parts);

    match CliParser::try_parse_from(parts) {
        Ok(v) => Ok(Some(v.command)),
        Err(e) => {
            println!("Unable to parse command\n'{e}'");
            Ok(None)
        }
    }
}

pub async fn process_config_update_prompt<L: LoggerBase, T, F>(logger: &L, prompt: &str, stdout: &mut Stdout, line: &mut Lines<BufReader<Stdin>>, default: T, curr: T, mut update: F) 
    where T: std::fmt::Display + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
    F: FnMut(T) -> () {
    
    let prompt = format!("{prompt} (Def: {}, Curr: {}): ", default, curr);
    match prompt_raw(&prompt, stdout, line).await {
        Ok(raw_curr) => {
            let trimmed = raw_curr.trim();
            if !trimmed.is_empty() {
                let as_num: T = match trimmed.parse() {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!(logger, "Invalid value: {e:?}");
                        println!("Invalid value.");
                        return;
                    }
                };

                update(as_num)
            }
        },
        Err(e) => {
            log_error!(logger, "Unable to get next line {e:?}");
            return;
        }
    }
}

pub async fn update_config<L: LoggerBase>(logger: &L, backend: &mut Backend, stdout: &mut Stdout, line: &mut Lines<BufReader<Stdin>>) {
    log_debug!(logger, "Getting previous configuration.");
    let raw_bytes = match backend.send_with_response(BackendRequests::GetConfig).await {
        Some(v) => match v {
            Ok(bytes) => bytes,
            Err(e) => {
                log_error!(logger, "Unable to retreive the previous configuration, erorr '{e:?}'");
                return;
            }
        },
        None => {
            log_error!(logger, "The backend was not able to serve the request.");
            return;
        }
    };
    let mut configuration: Configuration = match serde_json::from_slice(&raw_bytes) {
        Ok(v) => v,
        Err(e) => {
            log_error!(logger, "Unable to decode the previous configuration. '{e:?}'");
            return;
        }
    };

    println!("The configuration settings will be listed.\nTo leave a value unchanged, leave the line blank.\nOtherwise, insert your new value.\n");

    process_config_update_prompt(
        logger, 
        "# of maximum console connections", 
        stdout,
        line, 
        4, 
        configuration.max_console,
        |x| configuration.max_console = x
    ).await;
     process_config_update_prompt(
        logger, 
        "# of maximum client connections", 
        stdout,
        line, 
        6, 
        configuration.max_hosts,
        |x| configuration.max_hosts = x
    ).await;
    process_config_update_prompt(
        logger, 
        "port # for client connections", 
        stdout,
        line, 
        CLIENTS_PORT, 
        configuration.hosts_port,
        |x| configuration.hosts_port = x
    ).await;
    process_config_update_prompt(
        logger, 
        "metrics collection frequency (s)", 
        stdout,
        line, 
        3, 
        configuration.metric_freq,
        |x| configuration.metric_freq = x
    ).await;

    let new_setting_prompt = format!("Here are the new settings:\n{configuration:#?}\nAre you sure about these changes? (y/n) ");
    let do_save = {
        match prompt_raw(&new_setting_prompt, stdout, line).await {
            Ok(v) => v == "true" || v == "yes" || v == "y" || v == "Y",
            Err(_) => {
                log_error!(logger, "Unable to decode yes or no.");
                return;
            }
        }
    };

    if do_save {
        log_info!(logger, "Sending the new configuration back to the server.");
        
        if backend.send_with_response(BackendRequests::UpdateConfig(configuration)).await.is_some() {
            println!("New configuration saved.");
        }
        else {
            println!("The new configuration could not be saved.");
        }
    }

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
        let command = match prompt_command(&mut stdout, &mut lines).await? {
            Some(v) => v,
            None => continue
        };

        let request = match command {
            CliCommands::Quit => break,
            CliCommands::Clear => {
                print!("\x1B[2J\x1b[1;1H");
                stdout.flush().await.expect("unable to flush");
                continue;
            },
            CliCommands::Poll => BackendRequests::Poll,
            CliCommands::Config(config) => {
                if config == ConfigCommands::Update {
                    update_config(&logger, &mut backend, &mut stdout, &mut lines).await;
                    continue;
                }
                else {
                    config.into()
                }
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
                let message = match m {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!(&logger, "Unable to get the response due to error '{e}'");
                        continue;
                    }
                };

                match command {
                    CliCommands::Quit | CliCommands::Clear => unreachable!(),
                    CliCommands::Auth(inner) => print_auth_response(&logger, inner, &message),
                    CliCommands::Config(inner) => {
                        match inner {
                            ConfigCommands::Get => {
                                if cfg!(debug_assertions) {
                                    let as_string = String::from_utf8(message.clone()).expect("unable to decode the string");
                                    log_debug!(&logger, "As string: {}", &as_string);
                                }

                                let config_values: Option<Configuration> = match serde_json::from_slice(&message) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        log_error!(&logger, "Unable to decode the server's configurations. (error '{e:?}'");
                                        continue;
                                    }
                                };

                                if let Some(config) = config_values {
                                    println!("Console configuration:\n{config:#?}");
                                }
                                else {
                                    println!("The daemon's configuration is not loaded properly.");
                                }
                            },
                            ConfigCommands::Reload => println!("The daemon has been notified of the changed configuration."),
                            ConfigCommands::Update => todo!()
                        }
                    },
                    CliCommands::Poll => println!("The daemon is active.")
                }
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

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use chrono::{Utc,Duration};
    use common::{msg::{PendingUser, UserDetails, UserSummary}, user::UserHistoryElement};

    use crate::cli::{print_all_users_table, print_pending_users_table, print_user_history_table};

    #[test]
    fn table_printing() {
        // User history
        println!("USER HISTORY\n");
        let details = UserDetails::new(1, "Test User".to_string(), vec![
            UserHistoryElement::new(Ipv4Addr::new(127, 0, 0, 1).into(), Utc::now() - Duration::days(2)),
            UserHistoryElement::new(Ipv4Addr::new(127, 0, 0, 1).into(), Utc::now() - Duration::days(1) - Duration::hours(1))
        ]);
        print_user_history_table(details);

        println!("\nUSERS TABLE\n");
        let users = vec![
            UserSummary::new(1, "User One".to_string()),
            UserSummary::new(2, "User Two".to_string()),
            UserSummary::new(3, "User Three".to_string())
        ];
        print_all_users_table(users);

        println!("\nPENDING USERS TABLE\n");
        let pending_users = vec![
            PendingUser::new(1, Ipv4Addr::new(100, 140, 2, 3).into(), Utc::now() - Duration::minutes(4)),
            PendingUser::new(2, Ipv6Addr::new(4, 0xAB, 0x36, 0x32, 0xF1, 0x23, 0x34, 0x11).into(), Utc::now() - Duration::hours(1))
        ];
        print_pending_users_table(pending_users);
    }
}