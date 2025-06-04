use aes_gcm::{Aes256Gcm, KeyInit};
use rsa_ext::{
    PublicKey, RsaPrivateKey, RsaPublicKey, PaddingScheme, errors::Error as RsaError
};
use serde::{Deserialize, Serialize};

pub const RSA_BITS_SIZE: usize = 2048;

/// Represents a structure that supports RSA encryption. This is formed from a given `RsaPublicKey`. 
/// This is safe to pass between processes.
#[derive(Serialize, Deserialize)]
pub struct RsaEncryptor {
    key: RsaPublicKey
}
impl From<RsaPublicKey> for RsaEncryptor {
    fn from(value: RsaPublicKey) -> Self {
        Self {
            key: value
        }
    }
}
impl RsaEncryptor {
    /// Encrypts the given data using a random number generator. 
    pub fn encrypt<T>(&self, rng: &mut T, data: &[u8]) -> Result<Vec<u8>, RsaError> where T: rsa_ext::rand_core::RngCore + rsa_ext::rand_core::CryptoRng {
        let padding = PaddingScheme::new_oaep::<sha2::Sha256>();

        Ok( self.key.encrypt(rng, padding, data)? )
    }
}

/// Represents a key that has the ability to decrypt binary data.
/// Note that this struct is not serializeable / deserializable with Serde due to security concerns. 
/// This should not be shared between program instances. 
pub struct RsaDecryptor {
    key: RsaPrivateKey
}
impl From<RsaPrivateKey> for RsaDecryptor {
    fn from(value: RsaPrivateKey) -> Self {
        Self {
            key: value
        }
    }
}
impl RsaDecryptor {
    /// Creates a key from a given random number generator, using `RSA_BITS_SIZE` as the key size. 
    pub fn generate<T>(rng: &mut T) -> Result<Self, RsaError> where T: rsa_ext::rand_core::RngCore + rsa_ext::rand_core::CryptoRng {
        let key = RsaPrivateKey::new(rng, RSA_BITS_SIZE)?;

        Ok(
            Self {
                key
            }
        )
    }

    /// Creates a `RsaEncryptor` (public key) from the internal key. 
    pub fn make_public_key(&self) -> RsaEncryptor {
        let key = RsaPublicKey::from(&self.key);

        RsaEncryptor {
            key
        }
    }

    /// Decrypts binary data usingn the internal key. 
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, RsaError> {
        let padding = PaddingScheme::new_oaep::<sha2::Sha256>();

        Ok( self.key.decrypt(padding, data)? )
    }
}

/// A structure that handles the calls to encrypt and decrypt data with RSA. 
pub struct RsaDuplex {
    private: RsaPrivateKey,
    public: RsaPublicKey
}
impl From<RsaDuplex> for (RsaEncryptor, RsaDecryptor) {
    fn from(value: RsaDuplex) -> Self {
        ( value.public.into(), value.private.into() )
    }
}
impl RsaDuplex {
    /// Creates a new set of public and private keys for RSA, using `RSA_BITS_SIZE` as the default size. 
    pub fn new<T>(rng: &mut T) -> Result<Self, RsaError> where T: rsa_ext::rand_core::RngCore + rsa_ext::rand_core::CryptoRng {
        let private = RsaPrivateKey::new(rng, RSA_BITS_SIZE)?;
        let public = RsaPublicKey::from(&private);

        Ok(
            Self {
                private,
                public
            }
        )
    }

    /// Obtains a copy of the public key to use outside of the structure. 
    pub fn get_public(&self) -> RsaPublicKey {
        self.public.clone()
    }
    
    /// Encrypts a binary message using the public key. 
    pub fn encrypt<T>(&self, rng: &mut T, data: &[u8]) -> Result<Vec<u8>, RsaError> where T: rsa_ext::rand_core::RngCore + rsa_ext::rand_core::CryptoRng {
        let padding = PaddingScheme::new_oaep::<sha2::Sha256>();

        Ok( self.public.encrypt(rng, padding, data)? )
    }

    /// Decrypts a binary message using the private key. 
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, RsaError> {
        let padding = PaddingScheme::new_oaep::<sha2::Sha256>();

        Ok( self.private.decrypt(padding, data)? )
    }
}

pub struct AesDuplex {
    key: Aes256Gcm
}
impl AesDuplex {
    pub fn new() -> Result<Self, aes_gcm::Error> {
        let key = Aes256Gcm::generate_key(&mut aes_gcm::aead::OsRng);
        let cipher = Aes256Gcm::new(&key);

        todo!()
    }
}