
//! A module containing an abstraction for sending messages encrypted via RSA.
//! The stream is built over any inner stream, and any key. 
//! It gains functionality over the abilities of the stream and key.

use std::io::{Read, Write, Seek};
use std::marker::Unpin;
use std::pin::Pin;
use std::ops::Deref;

use tokio::io::{AsyncRead, AsyncWrite, AsyncSeek};
use rand_core::{RngCore, CryptoRng};
use serde::{Serialize, de::DeserializeOwned};

use super::super::encrypt::{RsaDecrypt, RsaEncrypt};
use super::err::{RsaRecvError, RsaSendError};

use exdisj::io::{
    net::{
        receive_buffer,
        receive_buffer_async,
        send_buffer,
        send_buffer_async
    }
};


/// A stream wrappers that automatically encrypts/decrypts messages sent using the RSA encryption algorithm.
/// Note that the structure of the RSA key itself will limit how many bytes can be sent at once.
/// This structure gains functionality based on the inner stream & key.
/// 
/// Here is a generalized list of those actions:
/// 
/// | `S` Restrictions           | `R` Restrictions | Action                                      | Data Types                                    | Extra Implementations
/// | -------------------------- | ---------------- | ------------------------------------------- | --------------------------------------------- | ---------------------
/// | None                       | None             | Get/get mut stream & key, take stream & key |                                               | 
/// | [`Read`]                   | [`RsaDecrypt`]   | Decrypt                                     | [`Vec<u8>`], [`String`], [`DeserializeOwned`] | 
/// | [`AsyncRead`] + [`Unpin`]  | [`RsaDecrypt`]   | Decrypt *async*                             | [`Vec<u8>`], [`String`], [`DeserializeOwned`] | 
/// | [`Write`]                  | [`RsaEncrypt`]   | Encrypt                                     | `[u8]`, [`str`], [`Serialize`]                | 
/// | [`AsyncWrite`] + [`Unpin`] | [`RsaEncrypt`]   | Encrypt *async*                             | `[u8]`, [`str`], [`Serialize`]                | 
/// | [`Seek`]                   | None             | Seek                                        |                                               | [`Seek`]
/// | [`AsyncSeek`] + [`Unpin`]  | [`Unpin`]        | Seek *async*                                |                                               | [`AsyncSeek`]
/// 
/// The extra implementations columns states that, depending on `S` and `R`, this structure will gain extra implementations.
/// 
/// # Async
/// Note that the futures used by this structure are dependent on the inner `S` stream. Therefore, if `S` is bound to a runtime, so is this structure.
/// Please review the documentation for `S` to know about possible panics.
/// 
/// If the inner stream supports async actions, then use the `*_async` variants. This is to prevent compile issues if the inner stream supports both sync & async operations.
#[derive(Debug)]
pub struct RsaStream<S, R> {
    inner: S,
    key: R
}
impl<S, R> RsaStream<S, R> {
    /// Constructs a new wrapper from a stream and key type.
    pub fn new(inner: S, key: R) -> Self {
        Self {
            inner,
            key
        }
    }

    /// Obtains the key from the wrapper.
    pub fn get_key(&self) -> &R {
        &self.key
    }
    /// Obtains a mutable reference from the wrapper.
    pub fn get_key_mut(&mut self) -> &mut R {
        &mut self.key
    }
    /// Obtains the inner stream from the wrapper.
    pub fn get_stream(&self) -> &S {
        &self.inner
    }
    /// Obtains a mutable reference to the inner wrapper.
    pub fn get_stream_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Takes the contents of the wrapper, yielding the inner stream and key.
    pub fn take(self) -> (S, R) {
        (self.inner, self.key)
    }
}


impl<S, R> RsaStream<S, R>
    where S: Read,
    R: RsaDecrypt {
        pub fn receive_bytes(&mut self) -> Result<Vec<u8>, RsaRecvError> {
            let mut result = vec![];
            receive_buffer(&mut result, &mut self.inner)
                .map_err(RsaRecvError::IO)?;

            //Decrypt
            self.key.decrypt(&result)
                .map_err(RsaRecvError::Decrypt)
        }

        pub fn receive_string(&mut self) -> Result<String, RsaRecvError> {
            let utf = self.receive_bytes()?;

            String::from_utf8(utf)
                .map_err(RsaRecvError::UTF)
        }

        pub fn receive_deserialize<T>(&mut self) -> Result<T, RsaRecvError> 
            where T: DeserializeOwned {
            let string = self.receive_bytes()?;

            serde_json::from_slice(&string)
                .map_err(RsaRecvError::Serde)
        }
}
impl<S, R> RsaStream<S, R> 
    where S: AsyncRead + Unpin,
    R: RsaDecrypt {
        pub async fn receive_bytes_async(&mut self) -> Result<Vec<u8>, RsaRecvError> {
            let mut result = vec![];
            receive_buffer_async(&mut result, &mut self.inner)
                .await
                .map_err(RsaRecvError::IO)?;

            //Decrypt
            self.key.decrypt(&result)
                .map_err(RsaRecvError::Decrypt)
        }

        pub async fn receive_string_async(&mut self) -> Result<String, RsaRecvError> {
            let utf = self.receive_bytes_async().await?;

            String::from_utf8(utf)
                .map_err(RsaRecvError::UTF)
        }

        pub async fn receive_deserialize_async<T>(&mut self) -> Result<T, RsaRecvError> 
            where T: DeserializeOwned {
            let string = self.receive_bytes_async().await?;

            serde_json::from_slice(&string)
                .map_err(RsaRecvError::Serde)
        }
}

impl<S, R> RsaStream<S, R> 
    where S: Write,
    R: RsaEncrypt {
        pub fn send_bytes<G>(&mut self, buff: &[u8], rng: &mut G) -> Result<(), RsaSendError> 
            where G: RngCore + CryptoRng {
                let encrypted = self.key.encrypt(buff, rng)
                    .map_err(RsaSendError::Encrypt)?;

                send_buffer(&encrypted, &mut self.inner)
                    .map_err(RsaSendError::IO)
        }
        pub fn send_bytes_deref<G, T>(&mut self, buff: &T, rng: &mut G) -> Result<(), RsaSendError>
            where G: RngCore + CryptoRng,
            T: Deref<Target = [u8]> + ?Sized {
                self.send_bytes(buff, rng)
            }
        pub fn send_string<G>(&mut self, string: &str, rng: &mut G) -> Result<(), RsaSendError> 
            where G: RngCore + CryptoRng {
                self.send_bytes(string.as_bytes(), rng)
            }

        pub fn send_serialize<G, T>(&mut self, target: &T, rng: &mut G) -> Result<(), RsaSendError>
            where G: RngCore + CryptoRng,
            T: Serialize + ?Sized {
                let as_string = serde_json::to_vec(target)
                    .map_err(RsaSendError::Serde)?;

                self.send_bytes(&as_string, rng)
            }
}
impl<S, R> RsaStream<S, R> 
    where S: AsyncWrite + Unpin,
    R: RsaEncrypt {
        pub async fn send_bytes_async<G>(&mut self, buff: &[u8], rng: &mut G) -> Result<(), RsaSendError> 
            where G: RngCore + CryptoRng {
                let encrypted = self.key.encrypt(buff, rng)
                    .map_err(RsaSendError::Encrypt)?;

                send_buffer_async(&encrypted, &mut self.inner)
                    .await
                    .map_err(RsaSendError::IO)
        }
        pub async fn send_bytes_deref_async<G, T>(&mut self, buff: &T, rng: &mut G) -> Result<(), RsaSendError>
            where G: RngCore + CryptoRng,
            T: Deref<Target = [u8]> + ?Sized {
                self.send_bytes_async(buff, rng).await
            }
        pub async fn send_string_async<G>(&mut self, string: &str, rng: &mut G) -> Result<(), RsaSendError> 
            where G: RngCore + CryptoRng {
                self.send_bytes_async(string.as_bytes(), rng).await
            }

        pub async fn send_serialize_async<G, T>(&mut self, target: &T, rng: &mut G) -> Result<(), RsaSendError>
            where G: RngCore + CryptoRng,
            T: Serialize + ?Sized {
                let as_string = serde_json::to_vec(target)
                    .map_err(RsaSendError::Serde)?;

                self.send_bytes_async(&as_string, rng).await
            }
}

impl<S, R> Seek for RsaStream<S, R> 
    where S: Seek {
        fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(pos)
        }
}
impl<S, R> AsyncSeek for RsaStream<S, R> 
    where S: AsyncSeek + Unpin,
    R: Unpin,
    Self: Unpin {
        fn start_seek(mut self: std::pin::Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
            let me: &mut RsaStream<S, R> = &mut self;
            let pinned_inner = Pin::new(&mut me.inner);
            pinned_inner.start_seek(position)
        }
        fn poll_complete(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<u64>> {
            let me: &mut RsaStream<S, R> = &mut self;
            let pinned_inner = Pin::new(&mut me.inner);
            pinned_inner.poll_complete(cx)
        }
}

#[cfg(test)]
mod tests {
    use std::io::{Seek, SeekFrom, Cursor};
    use rand::thread_rng;

    use super::*;
    use crate::auth::encrypt::rsa::RsaHandler;

    #[test]
    fn test_rsa_stream() {

        let inner_stream: Cursor<Vec<u8>> = Cursor::new(vec![]);
        let mut rng = thread_rng();

        let key = RsaHandler::new(&mut rng).expect("unable to make a key");
        let mut stream = RsaStream::new(inner_stream, key);

        let bytes = [100, 150, 200, 10];
        stream.send_bytes(&bytes, &mut rng).expect("Unable to send a message");
        stream.seek(SeekFrom::Start(0)).expect("unable to seek");

        let decoded = stream.receive_bytes().expect("Unable to get bytes back");

        assert_eq!(&decoded, &bytes)
    }

    #[test]
    fn test_rsa_stream_ref() {
        let inner_stream: Cursor<Vec<u8>> = Cursor::new(vec![]);
        let mut rng = thread_rng();

        let key = RsaHandler::new(&mut rng).expect("unable to make a key");

        let mut stream = RsaStream::new(inner_stream, &key);
        let bytes = [100, 150, 200, 10];
        stream.send_bytes(&bytes, &mut rng).expect("Unable to send a message");
        stream.seek(SeekFrom::Start(0)).expect("unable to seek");

        let decoded = stream.receive_bytes().expect("Unable to get bytes back");

        assert_eq!(&decoded, &bytes)
    }

    /*
    #[test]
    fn metrics_rsa_stream() {
        use std::io::Cursor;
        use chrono::{Utc, Duration};
        use rand::thread_rng;

        let inner_stream: Cursor<Vec<u8>> = Cursor::new(vec![]);
        let mut rng = thread_rng();

        let key = RsaHandler::new(&mut rng).expect("unable to make a key");
        let mut stream = RsaStream::new(inner_stream, key);

        //We define a set of variable size arrays to test encoding and decoding.
        let mut results: [(Duration, Duration); 6] = [(Duration::zero(), Duration::zero()); 6];
        let mut input: [Vec<u8>; 6] = [
            vec![0; 20],
            vec![0; 40],
            vec![0; 60],
            vec![0; 100],
            vec![0; 120],
            vec![0; 160]
        ]; 
        
        for array in &mut input {
            rng.fill_bytes(array)
        }

        for (i, data) in input.iter().enumerate() {
            // Reset the stream
            stream.get_stream_mut().set_position(0);
            stream.get_stream_mut().get_mut().clear();

            let start = Utc::now();
            stream.send_bytes(&data, &mut rng)
                .expect("Unable to send message.");

            let midpoint = Utc::now();
            let send_diff = midpoint - start;

            //Move to the front
            stream.get_stream_mut().set_position(0);

            let start = Utc::now();
            let result = stream.receive_bytes()
                .expect("Unable to recv message");
            let end = Utc::now();
            let recv_diff = end - start;

            assert_eq!(&result, data);
            results[i] = (send_diff, recv_diff);
        }

        println!("| {:^10} | {:^10} | {:^10} | {:^10} |", "Dataset", "Size (B)", "Send (ms)", "Receive(ms)");
        println!("| {:-^10} | {:-^10} | {:-^10} | {:-^10} |", "", "", "", "");

        for (i, (input_size, (send, recv))) in std::iter::zip(input.map(|x| x.len()), results).enumerate() {
            println!("| {:^10} | {:^10} | {:>10.5} | {:>10.5} |", i, input_size, send.as_seconds_f64() * 1000.0, recv.as_seconds_f64() * 1000.0);
        }
    }

     */
}