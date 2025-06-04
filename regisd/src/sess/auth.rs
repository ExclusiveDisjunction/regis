use std::str::Bytes;

use aes_gcm::{
    Aes256Gcm, Key
};
use chrono::{Utc, DateTime};
use rand_core::{CryptoRng, RngCore};
use rsa_ext::{
    PublicKey, RsaPrivateKey, RsaPublicKey, PaddingScheme, errors::Error as RsaError
};

pub const RSA_BITS_SIZE: usize = 2048;

pub trait EncryptionHandler : Sized { 
    type Error;

    fn new<T>(rng: &mut T) -> Result<Self, Self::Error> where T: RngCore + CryptoRng;
    fn encrypt<T>(&self, rng: &mut T, data: &[u8]) -> Result<Vec<u8>, Self::Error> where T: RngCore + CryptoRng;
}

struct RsaEncryptor {
    private: RsaPrivateKey,
    public: RsaPublicKey
}
impl EncryptionHandler for RsaEncryptor {
    type Error = RsaError;

    fn new<T>(rng: &mut T) -> Result<Self, RsaError> where T: RngCore + CryptoRng {
        let private = RsaPrivateKey::new(rng, RSA_BITS_SIZE)?;
        let public = RsaPublicKey::from(&private);

        Ok(
            Self {
                private,
                public
            }
        )
    }
    fn encrypt<T>(&self, rng: &mut T, data: &[u8]) -> Result<Vec<u8>, Self::Error> where T: RngCore + CryptoRng {
        let padding = PaddingScheme::new_oaep::<sha2::Sha256>();

        Ok( self.public.encrypt(rng, padding, data)? )
    }
}
impl RsaEncryptor {
    
}