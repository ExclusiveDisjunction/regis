use std::ops::Deref;
use std::io::{Read, Write};
use std::pin::Pin;
use std::string::FromUtf8Error;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use aes_gcm::{aead::{Aead, AeadCore, KeyInit, Nonce}, Aes256Gcm, Error as AesError, Key as AesKey};
use rsa_ext::{errors::Result as RsaResult, PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};
use rand_core::{RngCore, CryptoRng};
use serde::{Deserialize, Serialize};

use exdisj::io::net::{
    send_buffer,
    send_buffer_async, 
    receive_buffer,
    receive_buffer_async
};

pub type AesResult<T> = Result<T, AesError>;

pub trait RsaEncrypt {
    fn encrypt<R>(&self, msg: &[u8], rng: &mut R) -> RsaResult<Vec<u8>> 
        where R: CryptoRng + RngCore;
}
pub trait RsaDecrypt {
    fn decrypt(&self, msg: &[u8]) -> RsaResult<Vec<u8>>;
}

#[derive(Clone, Debug)]
pub struct RsaHandler {
    public: RsaPublicKey,
    private: RsaPrivateKey
}
impl RsaHandler {
    pub fn new<R>(rng: &mut R) -> RsaResult<Self> where R: CryptoRng + RngCore {
        let bits = 2048;
        let private = RsaPrivateKey::new(rng, bits).expect("failed to generate a key");
        let public = RsaPublicKey::from(&private);

        Ok(
            Self {
                public,
                private
            }
        )
        
    }

    #[inline]
    pub fn get_padding() -> PaddingScheme {
        PaddingScheme::new_oaep::<sha2::Sha256>()
    }
}
impl RsaEncrypt for RsaHandler {
    fn encrypt<R>(&self, msg: &[u8], rng: &mut R) -> RsaResult<Vec<u8>> where R: CryptoRng + RngCore {
        self.public.encrypt(rng, Self::get_padding(), msg)
    }
}
impl RsaDecrypt for RsaHandler {
    fn decrypt(&self, msg: &[u8]) -> RsaResult<Vec<u8>> {
        self.private.decrypt(Self::get_padding(), msg)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RsaEncrypter(RsaPublicKey);
impl RsaEncrypter {
    pub fn new(private: RsaPrivateKey) -> Self {
        Self(private.to_public_key())
    }
    pub fn new_direct(key: RsaPublicKey) -> Self {
        Self(key)
    }

    #[inline]
    pub fn get_padding() -> PaddingScheme {
        PaddingScheme::new_oaep::<sha2::Sha256>()
    }
}
impl RsaEncrypt for RsaEncrypter {
    fn encrypt<R>(&self, msg: &[u8], rng: &mut R) -> RsaResult<Vec<u8>> where R: CryptoRng + RngCore {
        self.0.encrypt(rng, Self::get_padding(), msg)
    }
}

pub struct AesHandler(AesKey<Aes256Gcm>);
impl AesHandler {
    pub fn new<R>(rng: &mut R) -> Self where R: CryptoRng + RngCore {
        let key = Aes256Gcm::generate_key(rng);

        Self(key)
    }

    pub fn encrypt<R>(&self, msg: &[u8], rng: &mut R) -> AesResult<(Vec<u8>, Nonce<Aes256Gcm>)> where R: CryptoRng + RngCore {
        let nonce = Aes256Gcm::generate_nonce(rng);
        let cipher = Aes256Gcm::new(&self.0);
        
        let encrypted = cipher.encrypt(&nonce, msg)?;

        Ok( ( 
            encrypted,
            nonce
        ) )
    }
    pub fn decrypt<R>(&self, msg: &[u8], nonce: &Nonce<Aes256Gcm>) -> AesResult<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.0);
        cipher.decrypt(&nonce, msg)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

pub struct RsaStream<S, R> {
    inner: S,
    key: R
}
impl<S, R> RsaStream<S, R> {
    pub fn new(inner: S, key: R) -> Self {
        Self {
            inner,
            key
        }
    }

    pub fn get_key(&self) -> &R {
        &self.key
    }
    pub fn get_key_mut(&mut self) -> &mut R {
        &mut self.key
    }
    pub fn get_stream(&self) -> &S {
        &self.inner
    }
    pub fn get_stream_mut(&mut self) -> &mut S {
        &mut self.inner
    }
}

#[derive(Debug)]
pub enum ReceiveError<T> {
    Encrypt(T),
    IO(std::io::Error),
    UTF(FromUtf8Error),
    Serde(serde_json::Error)
}

pub type RsaReceiveError = ReceiveError<rsa_ext::errors::Error>;
pub type AesReceiveError = ReceiveError<AesError>;

impl<S, R> RsaStream<S, R>
    where S: Read,
    R: RsaDecrypt {
        pub fn receive_bytes(&mut self) -> Result<Vec<u8>, RsaReceiveError> {
            let mut result = vec![];
            receive_buffer(&mut result, &mut self.inner)
                .map_err(RsaReceiveError::IO)?;

            //Decrypt
            self.key.decrypt(&result)
                .map_err(RsaReceiveError::Encrypt)
        }

        pub fn receive_string(&mut self) -> Result<String, RsaReceiveError> {
            let utf = self.receive_bytes()?;

            String::from_utf8(utf)
                .map_err(RsaReceiveError::UTF)
        }

        pub fn receive_deserialize<T>(&mut self) -> Result<T, RsaReceiveError> 
            where T: DeserializeOwned {
            let string = self.receive_string()?;

            serde_json::from_str(&string)
                .map_err(RsaReceiveError::Serde)
        }
}
impl<S, R> RsaStream<S, R> 
    where S: AsyncReadExt + Unpin,
    R: RsaDecrypt {
        pub async fn receive_bytes_async(&mut self) -> Result<Vec<u8>, RsaReceiveError> {
            let mut result = vec![];
            receive_buffer_async(&mut result, &mut self.inner)
                .await
                .map_err(RsaReceiveError::IO)?;

            //Decrypt
            self.key.decrypt(&result)
                .map_err(RsaReceiveError::Encrypt)
        }

        pub async fn receive_string_async(&mut self) -> Result<String, RsaReceiveError> {
            let utf = self.receive_bytes_async().await?;

            String::from_utf8(utf)
                .map_err(RsaReceiveError::UTF)
        }

        pub async fn receive_deserialize_async<T>(&mut self) -> Result<T, RsaReceiveError> 
            where T: DeserializeOwned {
            let string = self.receive_string_async().await?;

            serde_json::from_str(&string)
                .map_err(RsaReceiveError::Serde)
        }
}

#[derive(Debug)]
pub enum SendError<T> {
    Encrypt(T),
    IO(std::io::Error),
    Serde(serde_json::Error)
}

pub type RsaSendError = SendError<rsa_ext::errors::Error>;
pub type AesSendError = SendError<AesError>;

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
                self.send_bytes(&*buff, rng)
            }
        pub fn send_string<G>(&mut self, string: &str, rng: &mut G) -> Result<(), RsaSendError> 
            where G: RngCore + CryptoRng {
                self.send_bytes(string.as_bytes(), rng)
            }

        pub fn send_serialize<G, T>(&mut self, target: &T, rng: &mut G) -> Result<(), RsaSendError>
            where G: RngCore + CryptoRng,
            T: Serialize + ?Sized {
                let as_string = serde_json::to_string(target)
                    .map_err(RsaSendError::Serde)?;

                self.send_string(&as_string, rng)
            }
}
impl<S, R> RsaStream<S, R> 
    where S: AsyncWriteExt + Unpin,
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
                self.send_bytes_async(&*buff, rng).await
            }
        pub async fn send_string_async<G>(&mut self, string: &str, rng: &mut G) -> Result<(), RsaSendError> 
            where G: RngCore + CryptoRng {
                self.send_bytes_async(string.as_bytes(), rng).await
            }

        pub async fn send_serialize_async<G, T>(&mut self, target: &T, rng: &mut G) -> Result<(), RsaSendError>
            where G: RngCore + CryptoRng,
            T: Serialize + ?Sized {
                let as_string = serde_json::to_string(target)
                    .map_err(RsaSendError::Serde)?;

                self.send_string_async(&as_string, rng).await
            }
}

impl<S, R> std::io::Seek for RsaStream<S, R> 
    where S: std::io::Seek {
        fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(pos)
        }
}
impl<S, R> tokio::io::AsyncSeek for RsaStream<S, R> 
    where S: tokio::io::AsyncSeek + Unpin,
    R: Unpin,
    Self: Unpin {
        fn start_seek(mut self: std::pin::Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
            let me: &mut RsaStream<S, R> = &mut *self;
            let pinned_inner = Pin::new(&mut me.inner);
            pinned_inner.start_seek(position)
        }
        fn poll_complete(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<u64>> {
            let me: &mut RsaStream<S, R> = &mut *self;
            let pinned_inner = Pin::new(&mut me.inner);
            pinned_inner.poll_complete(cx)
        }
    }

#[cfg(test)]
mod tests {
    use std::io::{Seek, SeekFrom};

    use super::*;
    use crate::auth::encrypt::RsaHandler;

    #[test]
    fn test_rsa_stream() {
        use std::io::Cursor;
        use rand::thread_rng;

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
    fn metrics_rsa_stream() {
        use std::io::Cursor;
        use chrono::{DateTime, Utc, Duration};
        use rand::thread_rng;

        let inner_stream: Cursor<Vec<u8>> = Cursor::new(vec![]);
        let mut rng = thread_rng();

        let key = RsaHandler::new(&mut rng).expect("unable to make a key");
        let mut stream = RsaStream::new(inner_stream, key);

        //We define a set of variable size arrays to test encoding and decoding.
        let mut results: [(Duration, Duration); 6] = [(Duration::zero(), Duration::zero()); 6];
        let mut input: [Vec<u8>; 6] = [
            vec![0; 128],
            vec![0; 256],
            vec![0; 512],
            vec![0; 1024],
            vec![0; 2048],
            vec![0; 4096]
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

        println!("| {:^10} | {:^10} | {:^10} | {:^10} |", "Dataset", "Size (B)", "Send", "Receive");
        println!("| {:-^10} | {:-^10} | {:-^10} | {:-^10} |", "", "", "", "");

        for (i, (input_size, (send, recv))) in std::iter::zip(input.map(|x| x.len()), results).enumerate() {
            println!("| {:^10} | {:^10} | {:>10} | {:>10} |", i, input_size, send, recv);
        }
    }
}