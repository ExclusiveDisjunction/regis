use serde::{Serialize, Deserialize};

use rsa_ext::{errors::Result as RsaResult, PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};
use rand_core::{RngCore, CryptoRng};

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