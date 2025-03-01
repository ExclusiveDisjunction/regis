use serde::{Serialize, Deserialize};

use std::{fmt::{Debug, Display}, net::TcpStream};

use crate::net::NetworkError;

pub trait MessageBasis: Serialize + for<'de> Deserialize<'de> + PartialEq + Clone + Debug {

}
pub trait RequestMessage : MessageBasis {}
pub trait ResponseMessage : MessageBasis {}

/// A request with previous authorization to connect to the server
pub struct UserConnectRequest {

}

/// A request with no previous authorization to connect to the server
pub struct UserCreateRequest {

}

/// A response that shows an incorrect authorzation for a connection
pub struct BadAuthResponse {

}

/// A response that shows a correct connection authentication 
pub struct WelcomeResponse {
    
}

/// General inquiry about server status
pub struct ServerStatusRequest {

}
/// General response about server status
pub struct ServerStatusResponse {

}

/// Detailed inquiry about server status
pub struct ServerMetricsRequest {

}
/// Detailed response about server status 
pub struct ServerMetricsResponse {

}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum AckCode {
    Ok,
    Unauthorized,
    NotFound
}
impl Display for AckCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ok => "Ok",
                Self::Unauthorized => "Unauthorized",
                Self::NotFound => "Not Found"
            }
        )
    }
}

/// A general purpose response to some activity 
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Acknoledgement {
    code: AckCode,
    message: Option<String>
}
impl MessageBasis for Acknoledgement {} 
impl ResponseMessage for Acknoledgement { }
impl RequestMessage for Acknoledgement {}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RequestMessages {

}
impl MessageBasis for RequestMessages {}
impl RequestMessage for RequestMessages { }

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ResponseMessages {

}
impl MessageBasis for ResponseMessages { }
impl RequestMessage for ResponseMessages { }

pub fn send_request<T>(message: T, soc: TcpStream) -> Result<(), NetworkError>  where T: RequestMessage {
    todo!()
}
pub fn send_response<T>(message: T, soc: TcpStream) -> Result<(), NetworkError> where T: ResponseMessage {
    todo!()
}

pub fn decode_request<T>(soc: TcpStream) -> Result<T, NetworkError> where T: RequestMessage {
    todo!()
}
pub fn decode_response<T>(soc: TcpStream) -> Result<T, NetworkError> where T: ResponseMessage {
    todo!()
}