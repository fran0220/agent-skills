use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

const NONCE_SIZE: usize = 12;

/// Vault encrypts/decrypts credential values using AES-256-GCM.
pub struct Vault {
    cipher: Aes256Gcm,
}

impl Vault {
    /// Create a new Vault from a 32-byte key string.
    /// If the key is shorter, it will be zero-padded; if longer, truncated.
    pub fn new(key_str: &str) -> Self {
        let mut key_bytes = [0u8; 32];
        let src = key_str.as_bytes();
        let len = src.len().min(32);
        key_bytes[..len].copy_from_slice(&src[..len]);

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .expect("AES-256-GCM key init should not fail with 32 bytes");
        Self { cipher }
    }

    /// Encrypt a plaintext value. Returns (ciphertext, nonce).
    pub fn encrypt(&self, plaintext: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("encryption failed: {}", e))?;

        Ok((ciphertext, nonce_bytes.to_vec()))
    }

    /// Decrypt a ciphertext given its nonce.
    pub fn decrypt(&self, ciphertext: &[u8], nonce_bytes: &[u8]) -> anyhow::Result<String> {
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| anyhow::anyhow!("invalid utf8: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let vault = Vault::new("test-key-that-is-32-bytes-long!!");
        let plaintext = "sk-my-secret-api-key-12345";
        let (ct, nonce) = vault.encrypt(plaintext).unwrap();
        let decrypted = vault.decrypt(&ct, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
