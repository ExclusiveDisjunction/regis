use aes_gcm::{aead::{Aead, AeadCore, KeyInit, Nonce}, Aes256Gcm, Error as AesError, Key as AesKey};
use rand_core::{RngCore, CryptoRng};

pub type AesResult<T> = Result<T, AesError>;

#[derive(Debug)]
pub struct EncryptedAesMessage {
    cipher: Vec<u8>,
    nonce: Nonce<Aes256Gcm>
}
impl EncryptedAesMessage {
    pub fn new(cipher: Vec<u8>, nonce: Nonce<Aes256Gcm>) -> Self {
        Self {
            cipher,
            nonce
        }
    }
    
    pub fn cipher(&self) -> &[u8] {
        &self.cipher
    }
    pub fn nonce(&self) -> &Nonce<Aes256Gcm> {
        &self.nonce
    }


    pub fn take_cipher(self) -> Vec<u8> {
        self.cipher
    }
    pub fn take_nonce(self) -> Nonce<Aes256Gcm> {
        self.nonce
    }
    pub fn take(self) -> (Vec<u8>, Nonce<Aes256Gcm>) {
        (self.cipher, self.nonce)
    }
}

pub struct AesHandler(AesKey<Aes256Gcm>);
impl AesHandler {
    pub fn new<R>(rng: &mut R) -> Self where R: CryptoRng + RngCore {
        let key = Aes256Gcm::generate_key(rng);

        Self(key)
    }

    pub fn encrypt<R>(&self, msg: &[u8], rng: &mut R) -> AesResult<EncryptedAesMessage> where R: CryptoRng + RngCore {
        let nonce = Aes256Gcm::generate_nonce(rng);
        let cipher = Aes256Gcm::new(&self.0);
        
        let encrypted = cipher.encrypt(&nonce, msg)?;

        Ok( 
            EncryptedAesMessage::new( 
                encrypted,
                nonce
            ) 
        )
    }
    pub fn decrypt(&self, msg: &EncryptedAesMessage) -> AesResult<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.0);
        cipher.decrypt(msg.nonce(), msg.cipher())
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[test]
fn test_aes_handler() {
    let mut rng = rand::thread_rng();
    let handler = AesHandler::new(&mut rng);

    let mut message = [0u8; 256];
    rng.fill_bytes(&mut message);

    let encrypted = handler.encrypt(&message, &mut rng).expect("Unable to get the encrypted message");
    let decrypted = handler.decrypt(&encrypted).expect("unable to decrypt");

    assert_eq!(&decrypted, &message);
}
