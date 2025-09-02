//! Includes all the errors that can happen when sending encrypted messages.
//! To define these errors, we split them into two groups: **Sending** and **Receiving**.
//! ## Sending Errors:
//! 1. IO Error (Inner Stream) => [`std::io::Error`]
//! 2. Encryption Error (Encryption) => [`aes_gcm::Error`] or [`rsa_ext::errors::Error`]
//! 3. Serialization Error (Serde) => [`serde_json::Error`]
//! 
//! ## Receiving Errors:
//! 1. IO Error (Inner Stream) => [`std::io::Error`]
//! 2. Decryption Error (Encryption) => [`aes_gcm::Error`] or [`rsa_ext::errors::Error`]
//! 3. Decoding Error (Base64) => [`base64::DecodeError`]
//! 4. UTF-8 Decoding Error => [`std::string::FromUtf8Error`]
//! 5. Deserialization Error (Serde) => [`serde_json::Error`]
//! 6. ***AES-Only***: Invalid nonce (Decryption) -> [`InvalidNonceLengthError`] 
//! 
//! There are enums that represent these errors.

use std::fmt::{Debug, Display};
use std::io::Error as IOError;
use std::string::FromUtf8Error;
use std::error::Error as StdError;
use serde_json::Error as SerError;

use rsa_ext::errors::Error as RsaError;
use aes_gcm::Error as AesError;

/// An error that occurs when the sent nonce is not exactly 96 bits (12 bytes) long.
/// [`AesPacket`] returns this error while trying to decode into a [`EncryptedAesMessage`].
/// 
/// [`AesPacket`]: `super::aes_prelude::AesPacket<T>`
/// [`EncryptedAesMessage`]: `crate::auth::encrypt::EncryptedAesMessage`
#[derive(Debug)]
pub struct InvalidNonceLengthError;
impl Display for InvalidNonceLengthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the nonce is not exactly 96 bits, or 12 bytes")
    }
}

/// A generic wrapper around the errors that can occur when sending occurs.
/// Note that for specific operations, curtain errors are not possible. 
/// For example, when sending `&[u8]`, [`Serde`] is not possible since no serialization occurs.
/// The type parameter `T` represents the inner encryption error (such as [`RsaError`])
/// 
/// [`RsaError`]: `rsa_ext::errors::Error`
/// [`Serde`]: `SendError::Serde`
#[derive(Debug)]
pub enum SendError<T> {
    /// Represents the message could not be encrypted correctly.
    Encrypt(T),
    /// Represents that the internal stream failed to send the final `[u8]` buffer.
    IO(IOError),
    /// The data being sent could not be serialized with [`serde_json`].
    Serde(SerError)
}
impl<T> Display for SendError<T> where T: Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Display = match self {
            Self::Encrypt(v) => v,
            Self::IO(i) => i,
            Self::Serde(s) => s
        };

        x.fmt(f)
    }
}
impl<T> StdError for SendError<T> where T: Display + Debug { }

/// The standard type for RSA encryption/sending failures. See [`SendError`] for more information.
pub type RsaSendError = SendError<RsaError>;
/// The standard type for AES encryption/sending failures. See [`SendError`] for more information.
pub type AesSendError = SendError<AesError>;

/// Represents a failure to receive an encrypted message through the internal stream.
/// This can occur if:
/// 1. Unable to decrypt ([`Decrypt`])
/// 2. Unable to receive the buffer ([`IO`])
/// 3. Serde was unable to deserialize the value ([`Serde`])
/// 4. The bytes sent is not a valid UTF-8 sequence ([`UTF`])
/// 
/// **Note**: Case 4 can only happen when the bytes are being interpreted as strings, and case 3 can only happen if the data is deserialized by [`serde_json`].
/// 
/// [`Decrypt`]: `RsaRecvError::Decrypt`
/// [`IO`]: `RsaRecvError::IO`
/// [`Serde`]: `RsaRecvError::Serde`
/// [`UTF`]: `RsaRecvError::UTF`
#[derive(Debug)]
pub enum RsaRecvError {
    Decrypt(RsaError),
    IO(IOError),
    Serde(SerError),
    UTF(FromUtf8Error)
}
impl Display for RsaRecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Display = match self {
            Self::Decrypt(v) => v,
            Self::IO(v) => v,
            Self::Serde(v) => v,
            Self::UTF(v) => v
        };

        x.fmt(f)
    }
}
impl StdError for RsaRecvError { }

/// Represents a failure to receive an encrypted message through the internal stream.
/// This can occur if:
/// 1. Unable to decrypt ([`Decrypt`])
/// 2. Unable to receive the buffer ([`IO`])
/// 3. Serde was unable to deserialize the value ([`Serde`])
/// 4. The bytes sent is not a valid UTF-8 sequence ([`UTF`])
/// 5. The nonce sent was not long enough or too long ([`Nonce`])
/// 6. The message could not be decoded from base64 ([`Decode`])
/// 
/// **Note**: Case 4 can only happen when the bytes are being interpreted as strings, and case 3 can only happen if the data is deserialized by [`serde_json`].
/// 
/// [`Decrypt`]: `AesRecvError::Decrypt`
/// [`IO`]: `AesRecvError::IO`
/// [`Serde`]: `AesRecvError::Serde`
/// [`UTF`]: `AesRecvError::UTF`
/// [`Nonce`]: `AesRecvError::Nonce`
/// [`Decode`]: `AesRecvError::Decode`
#[derive(Debug)]
pub enum AesRecvError {
    Decrypt(AesError),
    IO(IOError),
    Serde(SerError),
    Nonce(InvalidNonceLengthError),
    Decode(base64::DecodeError),
    UTF(FromUtf8Error)
}
impl Display for AesRecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: &dyn Display = match self {
            Self::Decrypt(v) => v,
            Self::IO(v) => v,
            Self::Serde(v) => v,
            Self::Nonce(n) => n,
            Self::Decode(v) => v,
            Self::UTF(v) => v,
        };

        x.fmt(f)
    }
}
impl StdError for AesRecvError { }