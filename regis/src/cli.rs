use std::net::IpAddr;
use std::process::ExitCode;

use exdisj::auth::{AesRecvError, AesSendError, RsaRecvError};
use rand::{CryptoRng, RngCore};
use tokio::net::TcpStream;
use tokio::io::{stdin, stdout, AsyncWriteExt, AsyncBufReadExt, BufReader, Lines, Stdin, Stdout, AsyncRead, AsyncWrite};

use std::str::FromStr;
use std::io::Error as IOError;

use exdisj::{
    log_error, log_debug, log_critical,
    error::FormattingError,
    io::{
        log::Logger,
        lock::OptionRwProvider,
        net::{receive_buffer_async, send_buffer_async}
    },
    auth::{RsaHandler, RsaStream, AesStream, AesHandler}
};
use common::msg::{RequestMessages, ResponseMessages};
use rsa_ext::RsaPublicKey;

use crate::config::{KnownHost, CONFIG};
use crate::tool::connect as tool_connect;

pub async fn prompt(out: &mut Stdout, lines: &mut Lines<BufReader<Stdin>>) -> Result<String, IOError> {
    prompt_message(b"> ", out, lines).await
}
pub async fn prompt_message(msg: &[u8], out: &mut Stdout, lines: &mut Lines<BufReader<Stdin>>) -> Result<String, IOError> {
    out.write(msg).await?;
    out.flush().await?;

    let raw_command = lines.next_line()
        .await?
        .unwrap_or("quit".to_string());

    Ok( raw_command )
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
    match contents.trim() {
        "yes" | "true" | "y" => true,
        "no"  | "false" | "n" => false,
        _ => {
            println!("Unexpected argument '{contents}', assuming '{}'", if default { "yes" } else { "no" });
            default
        }
    }
}

#[derive(Debug)]
pub enum DestinationResolveError {
    Config,
    Quit,
    IO(IOError)
}

pub async fn resolve_from_known(lines: &mut Lines<BufReader<Stdin>>, out: &mut Stdout, logger: &Logger) -> Result<IpAddr, DestinationResolveError> {
    log_debug!(logger, "Getting known hosts");
    let lock = CONFIG.access();

    let hosts = match lock.access() {
        Some(v) => &v.hosts,
        None => {
            log_error!(logger, "Unable to access configuration.");
            return Err( DestinationResolveError::Config );
        }
    };

    loop {
        println!("Please choose a host to connect to:");
        for (i, host) in hosts.iter().enumerate() {
            println!("({}) - {}", i + 1, host);
        }
        println!();

        let raw_index = prompt_message(b"Index: ", out, lines).await
            .map_err(DestinationResolveError::IO)?
            .to_lowercase();

        if raw_index == "q" {
            log_error!(logger, "Aborting connection process, exiting.");
            return Err( DestinationResolveError::Quit );
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

        return Ok( *hosts[index - 1].addr() );
    }
}

pub async fn resolve_from_prompts(lines: &mut Lines<BufReader<Stdin>>, out: &mut Stdout, logger: &Logger) -> Result<IpAddr, DestinationResolveError> {
    let result: IpAddr;
    loop {
        let raw_addr = prompt_message(b"Please enter an IP address to connect to: ", out, lines).await  
            .map_err(DestinationResolveError::IO)?;

        if raw_addr == "q" || raw_addr == "Q" {
            log_error!(logger, "Aborting connection process, exiting.");
            return Err( DestinationResolveError::Quit );
        }

        result = match raw_addr.parse() {
            Ok(v) => v,
            Err(e) => {
                log_debug!(logger, "unable to parse host becuase of '{e}'");
                println!("Unable to parse the value. Try again, or type 'q' to abort.");
                continue;
            }
        };

        break;
    }

    let raw_adding = prompt_message(b"Add host to the known hosts? ", out, lines)
        .await
        .map_err(DestinationResolveError::IO)?;
    let adding = parse_bool(&raw_adding, false);

    if adding {
        log_debug!(logger, "Attempting to insert into known hosts.");
        let host_name = prompt_message(b"Please enter the host's name: ", out, lines).await
            .map_err(DestinationResolveError::IO)?;
        
        let to_insert = KnownHost::new(host_name, result);
        let mut lock = CONFIG.access_mut();
        match lock.access() {
            Some(v) => v.hosts.push(to_insert),
            None => {
                log_error!(logger, "Unable to access configuration for writing.");
                let raw_continue = prompt_message(b"Unable to write to configuration. Do you wish to continue program execution? ", out, lines).await
                    .map_err(DestinationResolveError::IO)?;
                let cont = parse_bool(&raw_continue, false);

                if !cont {
                    return Err( DestinationResolveError::Config );
                }
            }
        }
    }

    Ok( result )
}

pub async fn determine_dest_ip(lines: &mut Lines<BufReader<Stdin>>, out: &mut Stdout, logger: &Logger) -> Result<IpAddr, DestinationResolveError> {
    let raw = prompt_message(b"Has this host been connected to before? ", out, lines).await
        .map_err(DestinationResolveError::IO)?;
    let known = parse_bool(&raw, false);

    if known {
        resolve_from_known(lines, out, logger).await
    }
    else {
       resolve_from_prompts(lines, out, logger).await
    }
}

#[derive(Debug)]
pub enum ConnectionFailure {
    IO(IOError),
    Serde(serde_json::Error),
    Config,
    Quit,
    InvalidKey,
    RsaRecv(RsaRecvError)
}

pub async fn perform_handshake<R, S>(logger: &Logger, rng: &mut R, mut stream: S) -> Result<AesStream<S>, ConnectionFailure> 
    where S: AsyncRead + AsyncWrite + Unpin,
    R: RngCore + CryptoRng {
        let rsa_pub_priv = RsaHandler::new(rng).map_err(|x| {
            log_error!(logger, "Unable to create a RSA key, error '{x:?}'");
            return ConnectionFailure::InvalidKey
        })?;

        let (pub_key, priv_key) = rsa_pub_priv.split();
        let pub_key_as_bytes = match serde_json::to_vec(pub_key.public_key()) {
            Ok(v) => v,
            Err(e) => {
                log_error!(logger, "Unable to serialize the RSA public key, error '{:?}'", &e);
                return Err( ConnectionFailure::Serde(e) )
            }
        };

        log_debug!(logger, "Waiting for server RSA key.");
        let mut server_rsa_bytes: Vec<u8> = vec![];
        if let Err(e) = receive_buffer_async(&mut server_rsa_bytes, &mut stream).await {
            log_error!(logger, "Unable to receive the bytes for the server RSA key, error '{:?}'.", &e);
            return Err( ConnectionFailure::IO(e) )
        }

        let server_rsa: RsaPublicKey = match serde_json::from_slice(&server_rsa_bytes) {
            Ok(v) => v,
            Err(e) => {
                log_error!(logger, "Unable to deserialize the server's public RSA key, error: '{:?}'.", &e);
                return Err( ConnectionFailure::Serde(e) );
            }
        };
        log_debug!(logger, "Got server RSA key, sending client RSA key.");
        if let Err(e) = send_buffer_async(&pub_key_as_bytes, &mut stream).await {
            log_error!(logger, "Unable to send the public RSA key '{:?}'", &e);
            return Err( ConnectionFailure::IO(e) );
        }

        let complete_rsa = RsaHandler::from_parts(server_rsa, priv_key.into_inner());
        let mut rsa_stream = RsaStream::new(stream, complete_rsa);

        log_debug!(logger, "Waiting for server's AES key.");
        let aes_bytes: Vec<u8> = match rsa_stream.receive_bytes_async().await {
            Ok(v) => v,
            Err(e) => {
                log_error!(logger, "Unable to get the server's AES key bytes, error '{:?}'", &e);
                return Err( ConnectionFailure::RsaRecv(e) )
            }
        };
        let aes_key: AesHandler = match AesHandler::from_bytes(&aes_bytes) {
            Some(v) => v,
            None => {
                log_error!(logger, "Unable to decode the AES key.");
                return Err( ConnectionFailure::InvalidKey )
            }
        };

        // Now we can use AES encryption streams
        log_debug!(logger, "Switching to AES encrypted stream");
        Ok( AesStream::new(rsa_stream.take().0, aes_key) )
}

pub async fn connect<R>(lines: &mut Lines<BufReader<Stdin>>, out: &mut Stdout, logger: &Logger, rng: &mut R) -> Result<AesStream<TcpStream>, ConnectionFailure> 
    where R: RngCore + CryptoRng {
    let host = match determine_dest_ip(lines, out, logger).await {
        Ok(v) => v,
        Err(e) => {
            return Err(
                match e {
                    DestinationResolveError::Config => ConnectionFailure::Config,
                    DestinationResolveError::IO(x) => ConnectionFailure::IO(x),
                    DestinationResolveError::Quit => ConnectionFailure::Quit
                }
            )
        }
    };
    let stream = match tool_connect(host, logger).await {
        Ok(v) => v,
        Err(e) => {
            log_critical!(logger, "Unable to connect: '{:?}'", &e);
            return Err( ConnectionFailure::IO(e) );
        }
    };

    //Now the handshake
    perform_handshake(logger, rng, stream).await
}

#[derive(Debug)]
pub enum MainLoopFailure {
    IO(IOError),
    Send(AesSendError),
    Recv(AesRecvError)
}

pub async fn main_loop<R>(lines: &mut Lines<BufReader<Stdin>>, out: &mut Stdout, logger: &Logger, rng: &mut R, mut stream: AesStream<TcpStream>) -> Result<(), MainLoopFailure> 
    where R: RngCore + CryptoRng {
        println!("\n Type h or help for help, otherwise type commands.\n");

        loop {
            let raw_message = prompt(out, lines).await
                .map_err(MainLoopFailure::IO)?;

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

            if let Err(e) = stream.send_serialize_async(&message, rng).await {
                log_error!(logger, "Unable to send request to server '{:?}'", &e);
                return Err( MainLoopFailure::Send(e) );
            }

            let response: ResponseMessages = match stream.receive_deserialize_async().await {
                Ok(v) => v,
                Err(e) => {
                    log_error!(logger, "Unable to decode message from server '{:?}'", &e);
                    return Err( MainLoopFailure::Recv(e));
                }
            };

            match response {
                ResponseMessages::Ack(_) => (),
                ResponseMessages::Metrics(m) => {
                    println!("Current metrics:\n{m:#?}");
                }
                ResponseMessages::Status(s) => {
                    println!("Current status: {s:#?}");
                }
            }
        }
}

pub async fn cli_entry(logger: &Logger) -> Result<(), ExitCode> {    
    let stdin = BufReader::new(stdin());
    let mut stdout = stdout();
    let mut lines = stdin.lines();

    println!("Welcome to regis!");
    println!("  Version 0.1.0  ");
    println!("-----------------");

    println!("Please connect to a host.");

    let mut rng = rand::thread_rng();
    let connection = connect(&mut lines, &mut stdout, logger, &mut rng).await.map_err(|e| {
            log_error!(logger, "Unable to connect to a host '{:?}'", &e);
            return ExitCode::FAILURE;
        }
    )?;

    main_loop(&mut lines, &mut stdout, logger, &mut rng, connection).await.map_err(|x| {
        log_error!(logger, "Main loop exited with error '{x:?}'");
        return ExitCode::FAILURE
    })
}