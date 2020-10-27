use arrayref::array_ref;
use ed25519_dalek::{
    Keypair as EDKeypair, PublicKey as EDPublicKey, SecretKey as EDSecretKey, Signature, Signer,
    Verifier,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snafu::{ResultExt, Snafu};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

pub type PublicKey = EDPublicKey;
pub type SecretKey = EDSecretKey;
pub type Keypair = EDKeypair;

const DIGESTBYTES: usize = 32;

type Result<T, E = CryptoError> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum CryptoError {
    #[snafu(display("Signature verification error"))]
    SignatureVerificationError,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// Hash struct that mantains type semantics
pub struct HashOf<T> {
    hash: [u8; DIGESTBYTES],
    phantom: PhantomData<T>,
}

impl<T> Hash for HashOf<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl<T> HashOf<T>
where
    T: Serialize,
{
    pub fn new(to_hash: &T) -> Self {
        // Unwrap is safe because to_hash is serializable
        let json_string = serde_json::to_string(to_hash).unwrap();
        let hash = Sha256::digest(json_string.as_bytes());
        Self {
            hash: *array_ref!(&hash, 0, DIGESTBYTES),
            phantom: PhantomData,
        }
    }
}

/// Signature struct that maintains type semantics
#[derive(Debug, Clone)]
pub struct Signed<T> {
    signature: Signature,
    data: T,
}

impl<T> Signed<T>
where
    T: Serialize + Clone,
{
    pub fn new(to_sign: T, keypair: &Keypair) -> Self {
        // Unwrap is safe because to_hash is serializable
        let json_string = serde_json::to_string(&to_sign).unwrap();
        let signature = keypair.sign(json_string.as_bytes());
        Self {
            signature,
            data: to_sign,
        }
    }

    pub fn verify(&self, pk: &PublicKey) -> Result<()> {
        let json_string = serde_json::to_string(&self.data).unwrap();
        pk.verify(json_string.as_bytes(), &self.signature)
            .map_err(|_| CryptoError::SignatureVerificationError)?;
        Ok(())
    }

    pub fn get_data(&self) -> &T {
        &self.data
    }
}
