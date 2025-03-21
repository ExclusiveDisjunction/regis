use std::fmt::{Debug, Display};

use serde_json::Error as JsonError;

// Way to improve: Implement std::err for all of these things.

#[derive(PartialEq, Eq, Clone)]
pub struct ArgumentMissingError {
    arg: String
}
impl Debug for ArgumentMissingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "argument '{}' is missing & required", &self.arg)
    }
}
impl ArgumentMissingError {
    pub fn new<T: Into<String>>(arg: T) -> Self {
        Self {
            arg: arg.into()
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct ArgumentValueError {
    arg: String,
    value: String
}
impl Debug for ArgumentValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the value '{}' stored in argument '{}' is invalid", &self.value, &self.arg)
    }
}
impl ArgumentValueError {
    pub fn new<T: Into<String>, U: Debug>(arg: T, value: &U) -> Self {
        Self {
            arg: arg.into(),
            value: format!("{:?}", value)
        }
    }
    pub fn new_display<T: Into<String>, U: Display>(arg: T, value: &U) -> Self {
        Self {
            arg: arg.into(),
            value: value.to_string()
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct NullError {
    target: String
}
impl Debug for NullError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the target '{}' is null, and this is disallowed", &self.target)
    }
}
impl NullError {
    pub fn new<T: Into<String>>(target: T) -> Self {
        Self {
            target: target.into()
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct FormattingError {
    processed: String,
    reason: String
}
impl Debug for FormattingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the expression '{}' is invalid due to '{}'", &self.processed, &self.reason)
    }
}
impl FormattingError {
    pub fn new<T: ToString, U: Into<String>>(processed: &T, reason: U) -> Self {
        Self {
            processed: processed.to_string(),
            reason: reason.into()
        }
    }
}

#[derive(Clone)]
pub struct RangeError<T> {
    var: String,
    val: T,
    range: Option<(T, T)>
}
impl<T> PartialEq for RangeError<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.var == other.var && self.val == other.val && self.range == other.range
    }
}
impl<T> Eq for RangeError<T> where T: PartialEq + Eq { }
impl<T> Debug for RangeError<T> where T: std::fmt::Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.range.as_ref() {
            Some((a, b)) => write!(f, "the value '{}' is out of range ({} - {}) in the target '{}'", &self.val, a, b, &self.var),
            None => write!(f, "the value '{}' is out of range in the target '{}'", &self.val, &self.var)
        }
    }
}
impl<T> RangeError<T>  {
    pub fn new<S: Into<String>>(var: S, val: T, range: Option<(T, T)>) -> Self {
        Self {
            var: var.into(),
            val,
            range
        }
    }
}
pub type IndexRangeError = RangeError<usize>;

#[derive(PartialEq, Eq, Clone)]
pub struct PermissionError {
    resource: String
}
impl Debug for PermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the resource '{}' cannot be accessed due to lack of permissions", &self.resource)
    }
}
impl PermissionError {
    pub fn new<T: Into<String>>(resource: T) -> Self {
        Self {
            resource: resource.into()
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct OperationError {
    action: String,
    reason: String
}
impl Debug for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the operation '{}' is not allowed due to '{}'", &self.action, &self.action)
    }
}
impl OperationError {
    pub fn new<T: Into<String>, U: Into<String>>(action: T, reason: U) -> Self {
        Self {
            action: action.into(),
            reason: reason.into()
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct ConversionError {
    from: String,
    reason: String
}
impl Debug for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the value '{}' could not be converted due to '{}'", &self.from, &self.reason)
    }
}
impl ConversionError {
    pub fn new<T: Into<String>, U: Into<String>>(from: T, reason: U) -> Self {
        Self {
            from: from.into(),
            reason: reason.into()
        }
    }
    pub fn new_fmt<T: ToString, U: Into<String>>(from: &T, reason: U) -> Self {
        Self {
            from: from.to_string(),
            reason: reason.into()
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct UnexpectedError {
    reason: String
}
impl Debug for UnexpectedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "an unexpected error occured: '{}'", &self.reason)
    }
}
impl UnexpectedError {
    pub fn new<T: Into<String>>(reason: T) -> Self {
        Self {
            reason: reason.into()
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum NamingError {
    Empty,
    InvalidCharacters,
    TooLong,
    TooShort,
    Whitespace,
    StaringWithNumber,
    Scripting,
    FormatSpecifier,
    Address
}
impl Display for NamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self as &dyn Debug).fmt(f)
    }
}
impl Debug for NamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "the name provided was empty or just whitespace"),
            Self::InvalidCharacters => write!(f, "the name provided has invalid characters"),
            Self::TooLong => write!(f, "the name provided is too long"),
            Self::TooShort => write!(f, "the name provided is too short"),
            Self::Whitespace => write!(f, "the name contains whitespace internally. no spaces are allowed inside the name"),
            Self::StaringWithNumber => write!(f, "the name starts with a number, and this is not allowed"),
            Self::Scripting | Self::FormatSpecifier | Self::Address => write!(f, "the name looks like it is trying to do something it is not supposed to (scriptiong, format specifier, or address); this is disallowed")
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct PoisonError {
    message: String
}
impl Debug for PoisonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self as &dyn Display).fmt(f)
    }
}
impl Display for PoisonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "poisoned due to '{}'", &self.message)
    }
}
impl PoisonError {
    pub fn new<T: Display>(message: T) -> Self {
        Self {
            message: message.to_string()
        }
    }
}

pub enum Error {
    ArgVal(ArgumentValueError),
    ArgMiss(ArgumentMissingError),
    Null(NullError),
    Format(FormattingError),
    Range(IndexRangeError),
    Operation(OperationError),
    Conv(ConversionError),
    Unexpected(UnexpectedError),
    Name(NamingError)
}
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ArgVal(a), Self::ArgVal(b)) => a == b,
            (Self::ArgMiss(a), Self::ArgMiss(b)) => a == b,
            (Self::Null(a), Self::Null(b)) => a == b,
            (Self::Format(a), Self::Format(b)) => a == b,
            (Self::Range(a), Self::Range(b)) => a == b,
            (Self::Operation(a), Self::Operation(b)) => a == b,
            (Self::Conv(a), Self::Conv(b)) => a == b,
            (Self::Name(a), Self::Name(b)) => a == b,
            _ => false
        }
    }
}
impl Eq for Error { }
impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Debug = match self {
            Self::ArgVal(x) => x,
            Self::ArgMiss(x) => x,
            Self::Null(x) => x,
            Self::Format(f) => f,
            Self::Range(x) => x,
            Self::Operation(x) => x,
            Self::Conv(x) => x,
            Self::Unexpected(x) => x,
            Self::Name(x) => x
        };

        x.fmt(f)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self as &dyn Debug).fmt(f)
    }
}

impl From<ArgumentValueError> for Error {
    fn from(value: ArgumentValueError) -> Self {
        Self::ArgVal(value)
    }
}
impl From<ArgumentMissingError> for Error {
    fn from(value: ArgumentMissingError) -> Self {
        Self::ArgMiss(value)
    }
}
impl From<NullError> for Error {
    fn from(value: NullError) -> Self {
        Self::Null(value)
    }
}
impl From<FormattingError> for Error {
    fn from(value: FormattingError) -> Self {
        Self::Format(value)
    }
}
impl From<IndexRangeError> for Error {
    fn from(value: IndexRangeError) -> Self {
        Self::Range(value)
    }
}
impl From<OperationError> for Error {
    fn from(value: OperationError) -> Self {
        Self::Operation(value)
    }
}
impl From<ConversionError> for Error {
    fn from(value: ConversionError) -> Self {
        Self::Conv(value)
    }
}
impl From<UnexpectedError> for Error {
    fn from(value: UnexpectedError) -> Self {
        Self::Unexpected(value)
    }
}
impl From<NamingError> for Error {
    fn from(value: NamingError) -> Self {
        Self::Name(value)
    }
}

#[derive(Debug)]
pub enum IOError {
    IO(std::io::Error),
    Core(Error)
}
impl From<std::io::Error> for IOError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}
impl From<Error> for IOError {
    fn from(value: Error) -> Self {
        Self::Core(value)
    }
}
impl Display for IOError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Display = match self {
            Self::IO(x) => x,
            Self::Core(x) => x
        };

        x.fmt(f)
    }
}

/// Represents errors that come from Serde JSON or IO. 
#[derive(Debug)]
pub enum ParsingError {
    Serde(JsonError),
    IO(std::io::Error)
}
impl From<JsonError> for ParsingError {
    fn from(value: JsonError) -> Self {
        Self::Serde(value)
    }
}
impl From<std::io::Error> for ParsingError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}