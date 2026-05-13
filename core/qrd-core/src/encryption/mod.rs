//! Encryption module for AES-256-GCM per-column encryption.
//! 
//! Uses HKDF-SHA256 for key derivation from master key.
//! Each column chunk must use a fresh nonce.

use crate::error::{Error, Result};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::convert::TryFrom;

/// Encryption algorithm identifier.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionId {
    /// No encryption (ID: 0x00)
    None = 0x00,
    /// AES-256-GCM (ID: 0x01)
    Aes256Gcm = 0x01,
}

impl TryFrom<u8> for EncryptionId {
    type Error = Error;

    fn try_from(byte: u8) -> Result<Self> {
        match byte {
            0x00 => Ok(EncryptionId::None),
            0x01 => Ok(EncryptionId::Aes256Gcm),
            id => Err(Error::UnknownEncryption { id }),
        }
    }
}

/// AES-256-GCM cipher instance.
pub struct Cipher {
    key: [u8; 32],
}

impl Cipher {
    /// Creates a cipher from a 32-byte master key.
    pub fn new(master_key: &[u8; 32]) -> Self {
        Self {
            key: *master_key,
        }
    }

    /// Generates a new random 12-byte nonce.
    pub fn generate_nonce() -> [u8; 12] {
        let mut nonce = [0u8; 12];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut nonce);
        nonce
    }

    /// Derives a key from master key using HKDF-SHA256.
    pub fn derive_key(
        master_key: &[u8; 32],
        salt: Option<&[u8; 32]>,
        info: &[u8],
    ) -> [u8; 32] {
        let salt_ref = salt.map(|s| s.as_slice());
        let hkdf = Hkdf::<Sha256>::new(salt_ref, master_key);
        let mut key = [0u8; 32];
        hkdf.expand(info, &mut key)
            .expect("HKDF expand should not fail with 32-byte output");
        key
    }

    /// Encrypts plaintext using AES-256-GCM.
    /// Returns (ciphertext || auth_tag).
    pub fn encrypt(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::aead::generic_array::GenericArray;
        let cipher = Aes256Gcm::new(self.key.as_ref().into());
        let nonce_ga = GenericArray::from_slice(nonce);

        cipher
            .encrypt(nonce_ga, plaintext)
            .map_err(|_| Error::AuthenticationFailed)
    }

    /// Decrypts ciphertext using AES-256-GCM.
    /// Input is (ciphertext || auth_tag).
    pub fn decrypt(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::aead::generic_array::GenericArray;
        let cipher = Aes256Gcm::new(self.key.as_ref().into());
        let nonce_ga = GenericArray::from_slice(nonce);

        cipher
            .decrypt(nonce_ga, ciphertext)
            .map_err(|_| Error::AuthenticationFailed)
    }
}

/// Encrypts plaintext using AES-256-GCM.
/// Returns (nonce: 12B || ciphertext || auth_tag: 16B)
pub fn encrypt(master_key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
    let nonce = Cipher::generate_nonce();
    let cipher = Cipher::new(master_key);
    let ciphertext = cipher.encrypt(&nonce, plaintext)?;

    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypts ciphertext using AES-256-GCM.
/// Input is (nonce: 12B || ciphertext || auth_tag: 16B)
pub fn decrypt(master_key: &[u8; 32], ciphertext_blob: &[u8]) -> Result<Vec<u8>> {
    if ciphertext_blob.len() < 12 {
        return Err(Error::FileTooSmall { file_size: 0 });
    }

    let (nonce, encrypted_data) = ciphertext_blob.split_at(12);
    let mut nonce_array = [0u8; 12];
    nonce_array.copy_from_slice(nonce);

    let cipher = Cipher::new(master_key);
    cipher.decrypt(&nonce_array, encrypted_data)
}
