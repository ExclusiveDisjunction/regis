use std::io::Error as IOError;

use serde::de::DeserializeOwned;
use tokio::net::UnixStream;

use common::loc::COMM_PATH;
use common::msg::ConsoleRequests;
use exdisj::io::{msg::{decode_message_async, send_message_async, DecodeError, SendError}, net::receive_buffer_async};

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
        send_message_async(message.into(), &mut self.stream).await
    }
    pub async fn recv_bytes(&mut self) -> Result<Vec<u8>, DecodeError> {
        let mut result = vec![];
        receive_buffer_async(&mut result, &mut self.stream).await
            .map_err(DecodeError::IO)?;
        Ok(result)
    }
    pub async fn recv<T>(&mut self) -> Result<T, DecodeError> where T: DeserializeOwned {
        decode_message_async(&mut self.stream).await
    }
    pub async fn send_with_response<T, R>(&mut self, message: T) -> Result<R, ConnectionError>
        where T: Into<ConsoleRequests>,
        R: DeserializeOwned {
        self.send(message).await.map_err(ConnectionError::from)?;
        self.recv().await.map_err(ConnectionError::from)
    }
    pub async fn send_with_response_bytes<T>(&mut self, message: T) -> Result<Vec<u8>, ConnectionError>
        where T: Into<ConsoleRequests> {
        self.send(message).await.map_err(ConnectionError::from)?;
        self.recv_bytes().await.map_err(ConnectionError::from)
    }

    pub async fn poll(&mut self) -> Result<(), SendError> {
        self.send(ConsoleRequests::Poll).await
    }
    pub async fn kill(&mut self) -> Result<(), SendError> {
        self.send(ConsoleRequests::Shutdown).await
    }
}