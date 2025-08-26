use std::io::Error as IOError;

use tokio::net::UnixStream;

use common::{loc::COMM_PATH, msg::ConsoleResponses};
use common::msg::ConsoleRequests;
use exdisj::io::msg::{decode_response_async, send_request_async, DecodeError, SendError};

#[derive(Debug)]
pub enum ConnectionError {
    IO(IOError),
    Serde(serde_json::Error),
    UTF(std::string::FromUtf8Error),
    Inappropriate
}
impl From<IOError> for ConnectionError {
    fn from(value: IOError) -> Self {
        Self::IO(value)
    }
}
impl From<serde_json::Error> for ConnectionError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}
impl From<std::string::FromUtf8Error> for ConnectionError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::UTF(value)
    }
}
impl From<SendError> for ConnectionError {
    fn from(value: SendError) -> Self {
        match value {
            SendError::IO(i) => Self::IO(i),
            SendError::Serde(s) => Self::Serde(s)
        }
    }
}
impl From<DecodeError> for ConnectionError {
    fn from(value: DecodeError) -> Self {
        match value {
            DecodeError::IO(i) => Self::IO(i),
            DecodeError::Serde(s) => Self::Serde(s),
            DecodeError::UTF(u) => Self::UTF(u)
        }
    }
}

pub struct Connection {
    stream: UnixStream
}
impl Connection {
    pub async fn open() -> Result<Self, IOError> {
        let stream = UnixStream::connect(COMM_PATH).await?;

        Ok (
            Self {
                stream
            }
        )
    }

    pub async fn send<T>(&mut self, message: T) -> Result<(), SendError> where T: Into<ConsoleRequests> {
        send_request_async(message.into(), &mut self.stream).await
    }
    pub async fn recv(&mut self) -> Result<ConsoleResponses, DecodeError> {
        decode_response_async(&mut self.stream).await
    }
    pub async fn send_with_response<T>(&mut self, message: T) -> Result<ConsoleResponses, ConnectionError> where T: Into<ConsoleRequests> {
        self.send(message).await.map_err(ConnectionError::from)?;
        self.recv().await.map_err(ConnectionError::from)
    }

    pub async fn send_and_expect(&mut self, message: ConsoleRequests) -> Result<(), ConnectionError> {
        send_request_async(message, &mut self.stream).await.map_err(ConnectionError::from)?;

        match decode_response_async(&mut self.stream).await {
            Ok(v) => {
                if !matches!(v, ConsoleResponses::Ok) {
                    Err( ConnectionError::Inappropriate )
                }
                else {
                    Ok( () )
                }
            }
            Err(e) => Err( e.into() )
        }
    }
    

    pub async fn poll(&mut self) -> Result<(), ConnectionError> {
        self.send_and_expect(ConsoleRequests::Poll).await
    }
    pub async fn kill(&mut self) -> Result<(), ConnectionError> {
        self.send_and_expect(ConsoleRequests::Shutdown).await
    }
    pub async fn config(&mut self) -> Result<(), ConnectionError> {
        self.send_and_expect(ConsoleRequests::Config).await
    }
}