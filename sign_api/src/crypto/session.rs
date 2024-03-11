//! https://specs.walletconnect.com/2.0/specs/clients/sign/session-proposal

use {
    hkdf::Hkdf,
    rand::{rngs::OsRng, CryptoRng, RngCore},
    sha2::{Digest, Sha256},
    std::fmt::{Debug, Formatter},
    x25519_dalek::{EphemeralSecret, PublicKey},
};

/// Session key and topic derivation errors.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Failed to generate symmetric session key: {0}")]
    SymKeyGeneration(String),
}

#[derive(Clone)]
pub struct SessionKey {
    sym_key: [u8; 32],
    public_key: PublicKey,
}

impl Debug for SessionKey {
    /// Custom debug to hide the symmetrical key.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionKey")
            .field("sym_key", &"********")
            .field("public_key", &self.public_key)
            .finish()
    }
}

impl SessionKey {
    /// Creates new session key from `osrng`.
    ///
    /// Helper for  when `osrng` is good enough.
    pub fn from_osrng(sender_public_key: &[u8; 32]) -> Result<Self, SessionError> {
        SessionKey::diffie_hellman(OsRng, sender_public_key)
    }

    /// Performs Diffie-Hellman symmetric key derivation.
    pub fn diffie_hellman<T>(csprng: T, sender_public_key: &[u8; 32]) -> Result<Self, SessionError>
    where
        T: RngCore + CryptoRng,
    {
        let single_use_private_key = EphemeralSecret::random_from_rng(csprng);
        let public_key = PublicKey::from(&single_use_private_key);

        let ikm = single_use_private_key.diffie_hellman(&PublicKey::from(*sender_public_key));

        let mut session_sym_key = Self {
            sym_key: [0u8; 32],
            public_key,
        };
        let hk = Hkdf::<Sha256>::new(None, ikm.as_bytes());
        hk.expand(&[], &mut session_sym_key.sym_key)
            .map_err(|e| SessionError::SymKeyGeneration(e.to_string()))?;

        Ok(session_sym_key)
    }

    /// Gets symmetic key reference.
    pub fn symmetric_key(&self) -> &[u8; 32] {
        &self.sym_key
    }

    /// Gets "our" public key used in symmetric key derivation.
    pub fn diffie_public_key(&self) -> &[u8; 32] {
        self.public_key.as_bytes()
    }

    /// Generates new session topic.
    pub fn generate_topic(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.sym_key);
        hex::encode(hasher.finalize())
    }
}
