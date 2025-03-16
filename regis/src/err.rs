pub use common::error::*;

pub const CHECK_ERR_EXIT: u8 = 1;
pub const IO_ERR_EXIT: u8 = 2;
pub const LOG_ERR_EXIT: u8 = 3;
pub const NETWORK_ERR_EXIT: u8 = 4;
pub const WEIRD_ERR_EXIT: u8 = 5;
pub const CONFIG_ERR_EXIT: u8 = 6;
/// Describes when the user avoids doing something they have to. 
pub const AVOID_ERR_EXIT: u8 = 7;

#[derive(Debug)]
pub enum IOCommError {
    IO(std::io::Error),
    Core(Error)
}
impl From<std::io::Error> for IOCommError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}
impl From<Error> for IOCommError {
    fn from(value: Error) -> Self {
        Self::Core(value)
    }
}