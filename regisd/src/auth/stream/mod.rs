//! Wrappers around streams to send encrypted messages between hosts.
//! Use this module, along with the encryption handlers in [`crate::auth::encrypt`], to securley send information between hosts.

pub mod err;
pub mod aes_prelude;
pub mod aes;
pub mod rsa;

pub use err::*;
pub use aes_prelude::*;
pub use aes::*;
pub use rsa::*;