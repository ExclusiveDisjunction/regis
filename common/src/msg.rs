use serde::{Serialize, Deserialize};
use serde_json::{to_string, from_str};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::{
    fmt::{
        Debug, 
        Display}, 
    net::Ipv4Addr,
    string::FromUtf8Error};

use crate::net::{send_buffer, receive_buffer};

pub enum SendError {
    Serde(serde_json::Error),
    IO(std::io::Error)
}
impl From<serde_json::Error> for SendError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}
impl From<std::io::Error> for SendError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}
impl Debug for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Debug = match self {
            Self::Serde(v) => v,
            Self::IO(v) => v
        };

        x.fmt(f)
    }
}
impl Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Display = match self {
            Self::Serde(v) => v,
            Self::IO(v) => v
        };

        x.fmt(f)
    }
}

pub enum DecodeError {
    Serde(serde_json::Error),
    IO(std::io::Error),
    UTF(FromUtf8Error)
}
impl From<serde_json::Error> for DecodeError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}
impl From<std::io::Error> for DecodeError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}
impl From<FromUtf8Error> for DecodeError {
    fn from(value: FromUtf8Error) -> Self {
        Self::UTF(value)
    }
}
impl Debug for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Debug = match self {
            Self::Serde(v) => v,
            Self::IO(v) => v,
            Self::UTF(v) => v,
        };

        x.fmt(f)
    }
}
impl Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Display = match self {
            Self::Serde(v) => v,
            Self::IO(v) => v,
            Self::UTF(v) => v,
        };

        x.fmt(f)
    }
}

/// A collection of traits required for sending or receiving messages.
pub trait MessageBasis: Serialize + for<'de> Deserialize<'de> + PartialEq + Clone + Debug { }
/// A marker that this message is for requests.
pub trait RequestMessage : MessageBasis {}
/// A marker that this message is for responses.
pub trait ResponseMessage : MessageBasis {}

/// A request with previous authorization to connect to the server
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct SignInRequest {

}
impl MessageBasis for SignInRequest {}
impl RequestMessage for SignInRequest {}

/// A request with no previous authorization to connect to the server
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct CreateUserRequest {

}
impl MessageBasis for CreateUserRequest {}
impl RequestMessage for CreateUserRequest {}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ConnectRequest {
    Old(SignInRequest),
    New(CreateUserRequest)
}
impl From<CreateUserRequest> for ConnectRequest {
    fn from(value: CreateUserRequest) -> Self {
        Self::New(value)
    }
}
impl From<SignInRequest> for ConnectRequest {
    fn from(value: SignInRequest) -> Self {
        Self::Old(value)
    }
}
impl MessageBasis for ConnectRequest {}
impl RequestMessage for ConnectRequest {}

/// A response that shows an incorrect authorzation for a connection
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct BadAuthResponse {

}
impl MessageBasis for BadAuthResponse {}
impl ResponseMessage for BadAuthResponse {}

/// A response that shows a correct connection authentication 
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct WelcomeResponse {
    
}
impl MessageBasis for WelcomeResponse {}
impl ResponseMessage for WelcomeResponse {}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ConnectResponse {
    Fail(BadAuthResponse),
    Ok(WelcomeResponse)
}
impl From<BadAuthResponse> for ConnectResponse {
    fn from(value: BadAuthResponse) -> Self {
        Self::Fail(value)
    }
}
impl From<WelcomeResponse> for ConnectResponse {
    fn from(value: WelcomeResponse) -> Self {
        Self::Ok(value)
    }
}
impl MessageBasis for ConnectResponse {}
impl ResponseMessage for ConnectResponse {}

/// General inquiry about server status
pub struct ServerStatusRequest {

}
/// General response about server status
pub struct ServerStatusResponse {

}

/// A general request to be added to the status broadcast channel.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct MetricsSubscribeRequest {
    to_addr: Ipv4Addr,
    port: u16
}
impl MessageBasis for MetricsSubscribeRequest {}
impl RequestMessage for MetricsSubscribeRequest {}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[repr(u16)]
pub enum HttpCode {
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,
    EarlyHints = 103,
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    MultiStatus = 207,
    AlreadyReported = 208,
    ImUsed = 226,
    MultipleChoices = 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    PayloadTooLarge = 413,
    UriTooLong = 414,
    UnsupportedMediaType = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    ImATeapot = 418,
    MisdirectedRequest = 421,
    UnprocessableEntity = 422,
    Locked = 423,
    FailedDependency = 424,
    TooEarly = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HttpVersionNotSupported = 505,
    VariantAlsoNegotiates = 506,
    InsufficientStorage = 507,
    LoopDetected = 508,
    NotExtended = 510,
    NetworkAuthenticationRequired = 511,
}
impl Display for HttpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Continue => "Continue",
                Self::SwitchingProtocols => "Switching Protocols",
                Self::Processing => "Processing",
                Self::EarlyHints => "Early Hints",
                Self::Ok => "OK",
                Self::Created => "Created",
                Self::Accepted => "Accepted",
                Self::NonAuthoritativeInformation => "Non-Authoritative Information",
                Self::NoContent => "No Content",
                Self::ResetContent => "Reset Content",
                Self::PartialContent => "Partial Content",
                Self::MultiStatus => "Multi-Status",
                Self::AlreadyReported => "Already Reported",
                Self::ImUsed => "IM Used",
                Self::MultipleChoices => "Multiple Choices",
                Self::MovedPermanently => "Moved Permanently",
                Self::Found => "Found",
                Self::SeeOther => "See Other",
                Self::NotModified => "Not Modified",
                Self::UseProxy => "Use Proxy",
                Self::TemporaryRedirect => "Temporary Redirect",
                Self::PermanentRedirect => "Permanent Redirect",
                Self::BadRequest => "Bad Request",
                Self::Unauthorized => "Unauthorized",
                Self::PaymentRequired => "Payment Required",
                Self::Forbidden => "Forbidden",
                Self::NotFound => "Not Found",
                Self::MethodNotAllowed => "Method Not Allowed",
                Self::NotAcceptable => "Not Acceptable",
                Self::ProxyAuthenticationRequired => "Proxy Authentication Required",
                Self::RequestTimeout => "Request Timeout",
                Self::Conflict => "Conflict",
                Self::Gone => "Gone",
                Self::LengthRequired => "Length Required",
                Self::PreconditionFailed => "Precondition Failed",
                Self::PayloadTooLarge => "Payload Too Large",
                Self::UriTooLong => "URI Too Long",
                Self::UnsupportedMediaType => "Unsupported Media Type",
                Self::RangeNotSatisfiable => "Range Not Satisfiable",
                Self::ExpectationFailed => "Expectation Failed",
                Self::ImATeapot => "I'm a teapot",
                Self::MisdirectedRequest => "Misdirected Request",
                Self::UnprocessableEntity => "Unprocessable Entity",
                Self::Locked => "Locked",
                Self::FailedDependency => "Failed Dependency",
                Self::TooEarly => "Too Early",
                Self::UpgradeRequired => "Upgrade Required",
                Self::PreconditionRequired => "Precondition Required",
                Self::TooManyRequests => "Too Many Requests",
                Self::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
                Self::UnavailableForLegalReasons => "Unavailable For Legal Reasons",
                Self::InternalServerError => "Internal Server Error",
                Self::NotImplemented => "Not Implemented",
                Self::BadGateway => "Bad Gateway",
                Self::ServiceUnavailable => "Service Unavailable",
                Self::GatewayTimeout => "Gateway Timeout",
                Self::HttpVersionNotSupported => "HTTP Version Not Supported",
                Self::VariantAlsoNegotiates => "Variant Also Negotiates",
                Self::InsufficientStorage => "Insufficient Storage",
                Self::LoopDetected => "Loop Detected",
                Self::NotExtended => "Not Extended",
                Self::NetworkAuthenticationRequired => "Network Authentication Required",
            }
        )
    }
}

/// A general purpose response to some activity 
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Acknoledgement {
    code: HttpCode,
    message: Option<String>
}
impl MessageBasis for Acknoledgement {} 
impl ResponseMessage for Acknoledgement { }
impl RequestMessage for Acknoledgement {}
impl Acknoledgement {
    pub fn new(code: HttpCode, message: Option<String>) -> Self {
        Self {
            code,
            message
        }
    }

    pub fn code(&self) -> HttpCode {
        self.code
    }
    pub fn is_ok(&self) -> bool {
        matches!(self.code, HttpCode::Ok)
    }
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

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

pub async fn send_message<T, S>(message: T, sok: &mut S) -> Result<(), SendError> where T: MessageBasis, S: AsyncWriteExt + Unpin {
    let serialized = to_string(&message).map_err(SendError::from)?;
    send_buffer(serialized.as_bytes(), sok).await.map_err(SendError::from)
}
pub async fn send_request<T, S>(message: T, sok: &mut S) -> Result<(), SendError>  where T: RequestMessage, S: AsyncWriteExt + Unpin {
    send_message(message, sok).await
}
pub async fn send_response<T, S>(message: T, soc: &mut S) -> Result<(), SendError> where T: ResponseMessage, S: AsyncWriteExt + Unpin {
    send_message(message, soc).await
}

pub async fn decode_message<T, S>(soc: &mut S) -> Result<T, DecodeError> where T: MessageBasis, S: AsyncReadExt + Unpin {
    let mut contents: Vec<u8> = Vec::new();
    receive_buffer(&mut contents, soc).await.map_err(DecodeError::from)?;

    let str_contents = String::from_utf8(contents).map_err(DecodeError::from)?;
    let result: Result<T, _> = from_str(&str_contents);
   
    result.map_err(DecodeError::from)
}
pub async fn decode_request<T, S>(soc: &mut S) -> Result<T, DecodeError> where T: RequestMessage, S: AsyncReadExt + Unpin {
    decode_message(soc).await
}
pub async fn decode_response<T, S>(soc: &mut S) -> Result<T, DecodeError> where T: ResponseMessage, S: AsyncReadExt + Unpin {
    decode_message(soc).await
}