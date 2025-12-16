use std::process::ExitCode;
use std::io::Error as IOError;

use exdisj::{
    log_critical, 
    log_debug,
    log_error,
    log_info,
    log_warning,
    io::log::{
        Logger, 
        LoggerLevel, 
        ConstructableLogger,
        RedirectedLogger,
        LoggerRedirectConfiguration,
        OsLogger
    }
};
use tokio::{
    runtime::Runtime,
    io::{
        stdout,
        stdin,
        AsyncBufReadExt as _,
        BufReader,
        AsyncWriteExt as _,
        Lines,
        Stdin,
        Stdout
    }
};
use common::{
    config::DaemonConfig,
    usr::ClientUserInformation,
    msg::{
        ConsoleAuthRequests,
        PendingUser,
        UserDetails,
        UserSummary
    },
    regisc::{
        backend::{Backend, BackendRequests, DaemonConfigUpdate},
        conn::ConnectionError,
        REGISC_VERSION
    }
};
use clap::{Parser, ValueEnum};

pub fn cli_entry<L1, L2>(logger: L1, backend: L2) -> Result<(), ExitCode> 
where L1: ConstructableLogger, L2: ConstructableLogger + 'static,
L2::Err: std::error::Error + Send + Sync {
    // The CLI runs entirely in Tokio, so we need to create a runtime and run the entry.

    log_debug!(&logger, "Starting up tokio runtime");
    let runtime = match Runtime::new() {
        Ok(v) => v,
        Err(e) => {
            log_critical!(&logger, "Unable to start tokio runtime, so regisc cannot run. Error: {e:?}");
            return Err( ExitCode::FAILURE )
        }
    };
    let result = runtime.block_on(async move {
        async_cli_entry(logger, backend).await
    });

    if let Err(e) = result {
        eprintln!("Regisc completed with the error {e:?}");
        Err( ExitCode::FAILURE )
    }
    else {
        Ok( () )
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
        stdout.write_all(content.as_ref()).await?;
        stdout.flush().await?;

        let raw_command = lines.next_line()
            .await?
            .unwrap_or("quit".to_string());

        Ok( raw_command )
}
pub async fn prompt(stdout: &mut Stdout, lines: &mut Lines<BufReader<Stdin>>) -> Result<String, IOError> {
    prompt_raw("> ", stdout, lines).await
}

#[derive(Debug, Clone, clap::Subcommand, PartialEq, Eq)]
pub enum ConfigCommands {
    Reload,
    Get,
    Update(DaemonConfigUpdate)
}
impl From<ConfigCommands> for BackendRequests {
    fn from(value: ConfigCommands) -> BackendRequests {
        match value {
            ConfigCommands::Reload => BackendRequests::ReloadConfig,
            ConfigCommands::Get => BackendRequests::GetConfig,
            ConfigCommands::Update(v) => BackendRequests::UpdateConfig(v)
        }
    }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum AuthCommands {
    Pending,
    Revoke { id: u64 },
    Approve { id: u64, name: String },
    Users,
    History { id: u64 }
}
impl From<AuthCommands> for ConsoleAuthRequests {
    fn from(value: AuthCommands) -> ConsoleAuthRequests {
        match value {
            AuthCommands::Pending => ConsoleAuthRequests::Pending,
            AuthCommands::Users => ConsoleAuthRequests::AllUsers,
            AuthCommands::History { id } => ConsoleAuthRequests::UserHistory(id),
            AuthCommands::Approve { id, name} => ConsoleAuthRequests::Approve(id, name),
            AuthCommands::Revoke { id } => ConsoleAuthRequests::Revoke(id)
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

pub fn print_auth_approve_result<L: Logger>(logger: &L, name: &str, message: Option<ClientUserInformation>) {
    match message {
        Some(info) => println!("User with id {} ({name}) was approved.", info.id()),
        None => log_error!(logger, "The user '{name}' was not approved.")
    }
}
pub fn print_revoke_result<L: Logger>(logger: &L, id: u64, message: Option<bool>) {
    match message {
        Some(true) => println!("User with id {id} was revoked."),
        Some(false) => println!("User with id {id} could not be revoked (Perhaps it has a different id?)."),
        None => log_error!(logger, "Unable to decode the revoked status for user with id {id}")
    }
}

pub fn print_with_deserialization<'de, L: Logger, V: serde::Deserialize<'de>, F>(logger: &L, message: &'de [u8], inner: F) where F: FnOnce(V) {
    match serde_json::from_slice::<V>(message) {
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

pub fn print_auth_response<L: Logger>(logger: &L, inner: AuthCommands, message: &[u8]) {
    match inner {
        AuthCommands::Approve { id: _, name } => print_auth_approve_result(logger, &name, serde_json::from_slice(message).ok()),
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

pub async fn process_config_update_prompt<L: Logger, T, F>(logger: &L, prompt: &str, stdout: &mut Stdout, line: &mut Lines<BufReader<Stdin>>, default: T, curr: T, mut update: F) 
    where T: std::fmt::Display + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
    F: FnMut(T) {
    
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
        }
    }
}

pub async fn async_cli_entry<L1: ConstructableLogger, L2>(logger: L1, backend: L2) -> Result<(), MainLoopFailure> 
where L2::Err: std::error::Error + Send + Sync,
L2: 'static + ConstructableLogger  {
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

        let request = match &command {
            CliCommands::Quit => break,
            CliCommands::Clear => {
                print!("\x1B[2J\x1b[1;1H");
                stdout.flush().await.expect("unable to flush");
                continue;
            },
            CliCommands::Poll => BackendRequests::Poll,
            CliCommands::Config(config) => config.clone().into(),
            CliCommands::Auth(auth) => {
                BackendRequests::Auth(
                    auth.clone().into()
                )
            }
        };

        log_info!(&logger, "Sending request '{:?}' to backend", &request);
        match backend.send_with_response(request).await {
            Some(m) => {
                let message = match m {
                    Ok(v) => v,
                    Err(e) => {
                        log_error!(&logger, "Unable to get the response due to error '{e:?}'");
                        continue;
                    }
                };

                match command {
                    CliCommands::Quit | CliCommands::Clear => unreachable!(),
                    CliCommands::Auth(inner) => print_auth_response(&logger, inner, &message),
                    CliCommands::Config(inner) => {
                        match inner {
                            ConfigCommands::Get => {
                                let config_values: Option<DaemonConfig> = match serde_json::from_slice(&message) {
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
                            ConfigCommands::Update(_) => println!("The configuration has been updated.")
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
    use common::{msg::{PendingUser, UserDetails, UserSummary}, usr::UserHistoryElement};

    use super::*;

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
}

pub fn main() -> Result<(), ExitCode> {
    // Parse command
    let command = Options::parse();

    // Establish logger
    let level: LoggerLevel;
    let redirect: Option<LoggerLevel>;
    if cfg!(debug_assertions) || command.verbose {
        level = LoggerLevel::Debug;
        redirect = Some(LoggerLevel::Debug);
    }
    else {
        level = LoggerLevel::Info;
        redirect = None;
    }

    let inner_logger = match OsLogger::new("com.exdisj.regis.console", level, "Main".into(), ()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Unable to start the logger due to error: '{e:?}'");
            return Err( ExitCode::FAILURE );
        }
    };
    let stdout_redirect = if let Some(level) = redirect {
       LoggerRedirectConfiguration::new(std::io::stdout(), level, Some(LoggerLevel::Warning))
    }
    else {
        LoggerRedirectConfiguration::new_inactive(std::io::stdout())
    };

    let logger = RedirectedLogger::new_configured(inner_logger, stdout_redirect, Default::default());


    if let Some(q) = command.quick {
        log_info!(&logger, "Sending quick command {q:?}");

        return Ok( () ); 
    }

    log_info!(&logger, "Starting runtime");
    let (runtime_channel, end_channel) = match (logger.make_channel("Runtime".into()), logger.make_channel("User".into())) {
        (Ok(l1), Ok(l2)) => (l1, l2),
        (Ok(_), Err(e)) | (Err(e), Ok(_)) => {
            log_error!(&logger, "Unable to make one channel: '{e:?}'");
            return Err(ExitCode::FAILURE);
        },
        (Err(e1), Err(e2)) => {
            log_error!(&logger, "Unable to make both channels: '{e1:?}' and '{e2:?}'");
            return Err(ExitCode::FAILURE);
        }
    };

    if let Some(quick) = command.quick {
        panic!("Quick commands are not complete yet. Cannot complete {quick:?} request.");
    }

    // Now we do the CLI entry.
    cli_entry(end_channel, runtime_channel)?;

    log_info!(&logger, "Regisc complete.");
    Ok( () )
}
