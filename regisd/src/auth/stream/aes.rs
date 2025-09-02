use std::io::{Read, Write, Seek};
use std::marker::Unpin;
use std::pin::Pin;

use tokio::io::{AsyncRead, AsyncWrite, AsyncSeek};
use rand_core::{RngCore, CryptoRng};
use serde::{Serialize, de::DeserializeOwned};

use crate::auth::encrypt::AesHandler;

use super::err::{AesRecvError, AesSendError};
use super::aes_prelude::AesPacket;

use exdisj::io::{
    net::{
        receive_buffer,
        receive_buffer_async,
        send_buffer,
        send_buffer_async
    }
};

type RecvResult<T> = Result<T, AesRecvError>;
type SendResult = Result<(), AesSendError>;

pub struct AesStream<S> {
    inner: S,
    key: AesHandler
}
impl<S> AesStream<S> {
    pub fn new(inner: S, key: AesHandler) -> Self {
        Self {
            inner,
            key
        }
    }

    pub fn get_key(&self) -> &AesHandler {
        &self.key
    }
    pub fn get_key_mut(&mut self) -> &mut AesHandler {
        &mut self.key
    }
    pub fn get_inner(&self) -> &S {
        &self.inner
    }
    pub fn get_inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    pub fn take(self) -> (S, AesHandler) {
        (self.inner, self.key)
    }
}
impl<S> AesStream<S>
    where S: Read {
        pub fn receive_bytes(&mut self) -> RecvResult<Vec<u8>> {
            let mut result = vec![];
            receive_buffer(&mut result, &mut self.inner)
                .map_err(AesRecvError::IO)?;

            // Now that we have the bytes, we have to deserialize into a raw message
            let bundle: AesPacket<&str> = serde_json::from_slice(&result)
                .map_err(AesRecvError::Serde)?;

            let decoded = bundle.decode()
                .map_err(AesRecvError::Decode)?;

            let resolved = decoded.unwrap()
                .map_err(AesRecvError::Nonce)?;

            self.key.decrypt(&resolved)
                .map_err(AesRecvError::Decrypt)
        }

        pub fn receive_string(&mut self) -> RecvResult<String> {
            let utf = self.receive_bytes()?;

            String::from_utf8(utf)
                .map_err(AesRecvError::UTF)
        }

        pub fn receive_deserialize<T>(&mut self) -> RecvResult<T> 
            where T: DeserializeOwned {
            let string = self.receive_bytes()?;

            serde_json::from_slice(&string)
                .map_err(AesRecvError::Serde)
        }
}
impl<S> AesStream<S> 
    where S: AsyncRead + Unpin {
        pub async fn receive_bytes_async(&mut self) -> RecvResult<Vec<u8>> {
            let mut result = vec![];
            receive_buffer_async(&mut result, &mut self.inner)
                .await
                .map_err(AesRecvError::IO)?;

            // Now that we have the bytes, we have to deserialize into a raw message
            let bundle: AesPacket<&str> = serde_json::from_slice(&result)
                .map_err(AesRecvError::Serde)?;

            let decoded = bundle.decode()
                .map_err(AesRecvError::Decode)?;

            let resolved = decoded.unwrap()
                .map_err(AesRecvError::Nonce)?;

            self.key.decrypt(&resolved)
                .map_err(AesRecvError::Decrypt)
        }

        pub async fn receive_string_async(&mut self) -> RecvResult<String> {
            let utf = self.receive_bytes_async().await?;

            String::from_utf8(utf)
                .map_err(AesRecvError::UTF)
        }

        pub async fn receive_deserialize_async<T>(&mut self) -> RecvResult<T> 
            where T: DeserializeOwned {
            let string = self.receive_bytes_async().await?;

            serde_json::from_slice(&string)
                .map_err(AesRecvError::Serde)
        }
}
impl<S> AesStream<S> 
    where S: Write {
        pub fn send_bytes<G, B>(&mut self, buff: &B, rng: &mut G) -> SendResult 
            where G: RngCore + CryptoRng,
            B: AsRef<[u8]> + ?Sized {
                let encrypted = self.key.encrypt(buff.as_ref(), rng)
                    .map_err(AesSendError::Encrypt)?;

                let packet: AesPacket<Vec<u8>> = encrypted.into();
                let encoded = packet.encode();
                let bytes = serde_json::to_vec(&encoded)
                    .map_err(AesSendError::Serde)?;

                send_buffer(&bytes, &mut self.inner)
                    .map_err(AesSendError::IO)
                
        }

        pub fn send_serialize<G, T>(&mut self, target: &T, rng: &mut G) -> SendResult
            where G: RngCore + CryptoRng,
            T: Serialize + ?Sized {
                let as_string = serde_json::to_vec(target)
                    .map_err(AesSendError::Serde)?;

                self.send_bytes(&as_string, rng)
        }
}
impl<S> AesStream<S> 
    where S: AsyncWrite + Unpin {
        pub async fn send_bytes_async<G, B>(&mut self, buff: &B, rng: &mut G) -> SendResult 
            where G: RngCore + CryptoRng,
            B: AsRef<[u8]> + ?Sized {
                let encrypted = self.key.encrypt(buff.as_ref(), rng)
                    .map_err(AesSendError::Encrypt)?;

                let packet: AesPacket<Vec<u8>> = encrypted.into();
                let encoded = packet.encode();
                let bytes = serde_json::to_vec(&encoded)
                    .map_err(AesSendError::Serde)?;

                send_buffer_async(&bytes, &mut self.inner)
                    .await
                    .map_err(AesSendError::IO)
                
        }

        pub async fn send_serialize_async<G, T>(&mut self, target: &T, rng: &mut G) -> SendResult
            where G: RngCore + CryptoRng,
            T: Serialize + ?Sized {
                let as_string = serde_json::to_vec(target)
                    .map_err(AesSendError::Serde)?;

                self.send_bytes_async(&as_string, rng).await
        }
}
impl<S> Seek for AesStream<S> 
    where S: Seek {
        fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(pos)            
        }
}
impl<S> AsyncSeek for AesStream<S>
    where S: AsyncSeek + Unpin,
    Self: Unpin {
        fn start_seek(mut self: std::pin::Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
            let me: &mut AesStream<S> = &mut *self;
            let pinned_inner = Pin::new(&mut me.inner);
            pinned_inner.start_seek(position)
        }
        fn poll_complete(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<u64>> {
            let me: &mut AesStream<S> = &mut *self;
            let pinned_inner = Pin::new(&mut me.inner);
            pinned_inner.poll_complete(cx)
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::encrypt::AesHandler;
    use std::io::{Seek, SeekFrom, Cursor};
    use rand::thread_rng;

    #[test]
    fn test_aes_stream() {
        let inner_stream: Cursor<Vec<u8>> = Cursor::new(vec![]);
        let mut rng = thread_rng();

        let key = AesHandler::new(&mut rng);
        let mut stream = AesStream::new(inner_stream, key);

        let mut bytes = [0u8; 256];
        rng.fill_bytes(&mut bytes);

        stream.send_bytes(&bytes, &mut rng).expect("Unable to send a message");
        stream.seek(SeekFrom::Start(0)).expect("unable to seek");

        let decoded = stream.receive_bytes().expect("Unable to get bytes back");

        assert_eq!(&decoded, &bytes)
    }
}