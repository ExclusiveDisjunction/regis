use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Nonce, Key as AesKey, Error as AesError
};
use rsa_ext::{errors::Result as RsaResult, PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};
use rand_core::{RngCore, CryptoRng};
use serde::{Deserialize, Serialize};

pub type AesResult<T> = Result<T, AesError>;

#[derive(Serialize, Deserialize, Clone, Debug)]
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

    pub fn encrypt<R>(&self, msg: &[u8], rng: &mut R) -> RsaResult<Vec<u8>> where R: CryptoRng + RngCore {
        self.public.encrypt(rng, Self::get_padding(), msg)
    }
    pub fn decrypt(&self, msg: &[u8]) -> RsaResult<Vec<u8>> {
        self.private.decrypt(Self::get_padding(), msg)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RsaEncrypter(RsaPublicKey);
impl RsaEncrypter {
    #[inline]
    pub fn get_padding() -> PaddingScheme {
        PaddingScheme::new_oaep::<sha2::Sha256>()
    }

    pub fn encrypt<R>(&self, msg: &[u8], rng: &mut R) -> RsaResult<Vec<u8>> where R: CryptoRng + RngCore {
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
        let nonce: Nonce<_> = Aes256Gcm::generate_nonce(rng);
        let cipher = Aes256Gcm::new(&self.0);
        
        let encrypted = cipher.encrypt(&nonce, msg)?;

        Ok( ( 
            encrypted,
            nonce
        ) )
    }
}