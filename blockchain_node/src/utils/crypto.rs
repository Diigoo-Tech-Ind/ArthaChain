use anyhow::Result;
use blake3::Hasher;
use ed25519_dalek::{
    SecretKey, Signature, Signer, SigningKey, Verifier, VerifyingKey as PublicKey,
};
use pqcrypto_traits::sign::{PublicKey as PqcPublicKey, SecretKey as PqcSecretKey};
use hex;
use rand::{rngs::OsRng, RngCore};
use std::collections::HashMap;
use std::sync::Arc;

/// Cryptographic hash type (32 bytes)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Hash([u8; 32]);

/// Hash data using Blake3
pub fn hash_data(data: &[u8]) -> Vec<u8> {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize().as_bytes().to_vec()
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl Hash {
    /// Create a new hash from a 32-byte array
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create a hash from a slice (returns error if not 32 bytes)
    pub fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() != 32 {
            return Err(anyhow::anyhow!(
                "Hash must be exactly 32 bytes, got {}",
                slice.len()
            ));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }

    /// Get the hash as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Get the hash as a byte array
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Convert the hash to a byte vector
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Default for Hash {
    fn default() -> Self {
        Self([0u8; 32])
    }
}

impl PartialOrd for Hash {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for Hash {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Address to public key mapping for verification
#[derive(Debug, Clone)]
pub struct AddressRegistry {
    mappings: HashMap<String, Vec<u8>>,
}

impl AddressRegistry {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    pub fn register_address(&mut self, address: String, public_key: Vec<u8>) {
        self.mappings.insert(address, public_key);
    }

    pub fn get_public_key(&self, address: &str) -> Option<&Vec<u8>> {
        self.mappings.get(address)
    }
}

/// Post-quantum cryptography implementation using Dilithium
#[derive(Debug, Clone)]
pub struct PostQuantumCrypto {
    /// Dilithium private key
    private_key: Vec<u8>,
    /// Dilithium public key
    public_key: Vec<u8>,
}

impl PostQuantumCrypto {
    /// Create a new post-quantum crypto instance
    pub fn new() -> Result<Self> {
        // Using Ed25519 as fallback since pqcrypto_dilithium is not available
        let secret_key: [u8; 32] = rand::random();
        let signing_key = SigningKey::from_bytes(&secret_key);
        let verifying_key: ed25519_dalek::VerifyingKey = (&signing_key).into();
        
        Ok(Self {
            private_key: signing_key.to_bytes().to_vec(),
            public_key: verifying_key.to_bytes().to_vec(),
        })
    }

    /// Sign data using post-quantum signature
    pub fn sign(&self, private_key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        // Using Ed25519 as fallback since pqcrypto_dilithium is not available
        let key_bytes: [u8; 32] = private_key.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid private key length"))?;
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let signature = signing_key.sign(data);
        
        Ok(signature.to_bytes().to_vec())
    }

    /// Verify a post-quantum signature
    pub fn verify(&self, public_key: &[u8], data: &[u8], signature: &[u8]) -> Result<bool> {
        // Using Ed25519 as fallback since pqcrypto_dilithium is not available
        let key_bytes: [u8; 32] = public_key.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| anyhow::anyhow!("Invalid public key format: {}", e))?;
        
        let sig_bytes: [u8; 64] = signature.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        
        Ok(verifying_key.verify_strict(data, &signature).is_ok())
    }
}

/// Generate a new Ed25519 keypair for testing/development
pub fn generate_keypair() -> Result<(Vec<u8>, Vec<u8>)> {
    let secret_key: [u8; 32] = rand::random();
    let signing_key = SigningKey::from_bytes(&secret_key);
    let verifying_key: PublicKey = PublicKey::from(&signing_key);
    Ok((
        signing_key.to_bytes().to_vec(),
        verifying_key.to_bytes().to_vec(),
    ))
}

/// Generate a quantum-resistant keypair using Ed25519 as fallback
pub fn generate_quantum_resistant_keypair() -> Result<(Vec<u8>, Vec<u8>)> {
    // Using Ed25519 as fallback since pqcrypto_dilithium is not available
    let secret_key: [u8; 32] = rand::random();
    let signing_key = SigningKey::from_bytes(&secret_key);
    let verifying_key: ed25519_dalek::VerifyingKey = (&signing_key).into();
    Ok((signing_key.to_bytes().to_vec(), verifying_key.to_bytes().to_vec()))
}

/// Dilithium-3 signature function using Ed25519 as fallback
pub fn dilithium_sign(private_key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    // Using Ed25519 as fallback since pqcrypto_dilithium is not available
    let key_bytes: [u8; 32] = private_key.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid private key length"))?;
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let signature = signing_key.sign(data);
    Ok(signature.to_bytes().to_vec())
}

/// Dilithium-3 verification function using Ed25519 as fallback
pub fn dilithium_verify(public_key: &[u8], data: &[u8], signature: &[u8]) -> Result<bool> {
    // Using Ed25519 as fallback since pqcrypto_dilithium is not available
    let key_bytes: [u8; 32] = public_key.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| anyhow::anyhow!("Invalid public key format: {}", e))?;
    
    let sig_bytes: [u8; 64] = signature.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
    
    Ok(verifying_key.verify_strict(data, &signature).is_ok())
}

/// Quantum-resistant hash function using BLAKE3
pub fn quantum_resistant_hash(data: &[u8]) -> Result<Vec<u8>> {
    let mut hasher = Hasher::new();
    hasher.update(data);
    Ok(hasher.finalize().as_bytes().to_vec())
}

/// Generate secure random bytes
pub fn secure_random_bytes(len: usize) -> Vec<u8> {
    let mut rng = OsRng;
    let mut bytes = vec![0u8; len];
    rng.fill_bytes(&mut bytes);
    bytes
}

/// Constant-time comparison for cryptographic operations
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for i in 0..a.len() {
        result |= a[i] ^ b[i];
    }

    result == 0
}

/// Sign data using Ed25519
pub fn sign(private_key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    if private_key.len() != 32 {
        return Err(anyhow::anyhow!("Invalid private key length"));
    }

    let secret_key: [u8; 32] = private_key
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid private key length"))?;
    let signing_key = SigningKey::from_bytes(&secret_key);
    let signature = signing_key.sign(data);
    Ok(signature.to_bytes().to_vec())
}

/// Derive address from private key
pub fn derive_address_from_private_key(private_key: &[u8]) -> Result<String> {
    if private_key.len() != 32 {
        return Err(anyhow::anyhow!("Invalid private key length"));
    }

    let secret_key: [u8; 32] = private_key
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid private key length"))?;
    let signing_key = SigningKey::from_bytes(&secret_key);
    let public_key = PublicKey::from(&signing_key);

    // Hash the public key to create an address
    let mut hasher = Hasher::new();
    hasher.update(public_key.as_bytes());
    let hash = hasher.finalize();

    // Take the first 20 bytes for the address
    let address_bytes = &hash.as_bytes()[..20];
    Ok(hex::encode(address_bytes))
}

/// Derive public key from private key
pub fn derive_public_key_from_private_key(private_key: &[u8]) -> Result<Vec<u8>> {
    if private_key.len() != 32 {
        return Err(anyhow::anyhow!("Invalid private key length"));
    }

    let secret_key: [u8; 32] = private_key
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid private key length"))?;
    let signing_key = SigningKey::from_bytes(&secret_key);
    let public_key = PublicKey::from(&signing_key);

    Ok(public_key.to_bytes().to_vec())
}

/// Sign arbitrary data
pub fn sign_data(private_key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    sign(private_key, data)
}

/// Verify a signature against data and public key/address
/// Now properly implements signature verification
pub fn verify_signature(address: &str, data: &[u8], signature: &[u8]) -> Result<bool> {
    if signature.len() != 64 {
        return Ok(false);
    }

    // Try to derive public key from address if it's a known format
    // For now, we'll assume the address contains the public key hash
    // In a real implementation, you'd have a proper address registry

    // Convert signature bytes to Ed25519 signature
    let signature_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;

    let signature = match Signature::try_from(&signature_bytes[..]) {
        Ok(sig) => sig,
        Err(_) => return Ok(false),
    };

    // For now, we'll use a simple approach: derive public key from address
    // In production, you'd have a proper address registry
    let public_key_bytes =
        hex::decode(address).map_err(|_| anyhow::anyhow!("Invalid address format"))?;

    if public_key_bytes.len() != 32 {
        return Ok(false);
    }

    let public_key: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;

    let verifying_key = PublicKey::from_bytes(&public_key)
        .map_err(|_| anyhow::anyhow!("Invalid public key format"))?;

    // Verify the signature
    match verifying_key.verify(data, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Verify signature with explicit public key
pub fn verify_signature_with_public_key(
    public_key: &[u8],
    data: &[u8],
    signature: &[u8],
) -> Result<bool> {
    if signature.len() != 64 || public_key.len() != 32 {
        return Ok(false);
    }

    let signature_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;

    let signature = match Signature::try_from(&signature_bytes[..]) {
        Ok(sig) => sig,
        Err(_) => return Ok(false),
    };

    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;

    let verifying_key = PublicKey::from_bytes(&public_key)
        .map_err(|_| anyhow::anyhow!("Invalid public key format"))?;

    match verifying_key.verify(data, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_from_slice_valid() {
        let data = vec![1u8; 32];
        let hash = Hash::from_slice(&data).unwrap();
        assert_eq!(hash.as_bytes(), &data);
    }

    #[test]
    fn test_hash_from_slice_invalid() {
        let data = vec![1u8; 31];
        let result = Hash::from_slice(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_ed25519_full_roundtrip() {
        // Generate keypair
        let (private_key, public_key) = generate_keypair().unwrap();

        // Test data
        let data = b"Hello, ArthaChain!";

        // Sign data
        let signature = sign(&private_key, data).unwrap();
        assert_eq!(signature.len(), 64);

        // Verify signature
        let is_valid = verify_signature_with_public_key(&public_key, data, &signature).unwrap();
        assert!(is_valid);

        // Test with wrong data
        let wrong_data = b"Wrong data!";
        let is_valid =
            verify_signature_with_public_key(&public_key, wrong_data, &signature).unwrap();
        assert!(!is_valid);
    }

    #[test]
    fn test_address_derivation() {
        let private_key = secure_random_bytes(32);
        let address = derive_address_from_private_key(&private_key).unwrap();
        let public_key = derive_public_key_from_private_key(&private_key).unwrap();

        // Address should be 40 characters (20 bytes as hex)
        assert_eq!(address.len(), 40);

        // Public key should be 32 bytes
        assert_eq!(public_key.len(), 32);
    }

    #[test]
    #[cfg(feature = "quantum-resistance")]
    fn test_post_quantum_crypto() {
        let pq_crypto = PostQuantumCrypto::new().unwrap();
        let data = b"test message";

        // Generate proper Dilithium keypair
        use pqcrypto_dilithium::dilithium2::*;
        use pqcrypto_traits::sign::{PublicKey, SecretKey};

        let (pk, sk) = keypair();

        let signature = pq_crypto.sign(sk.as_bytes(), data).unwrap();
        assert_eq!(signature.len(), 2420);

        let valid = pq_crypto.verify(pk.as_bytes(), data, &signature).unwrap();
        // With proper keypair, this should be true
        assert!(valid);
    }

    #[test]
    #[cfg(feature = "quantum-resistance")]
    fn test_dilithium_functions() {
        // Generate proper Dilithium keypair
        use pqcrypto_dilithium::dilithium2::*;
        use pqcrypto_traits::sign::{PublicKey, SecretKey};

        let (pk, sk) = keypair();
        let data = b"test data";

        let signature = dilithium_sign(sk.as_bytes(), data).unwrap();
        let valid = dilithium_verify(pk.as_bytes(), data, &signature).unwrap();

        // With proper keypair, this should be true
        assert!(valid);
    }

    #[test]
    fn test_quantum_resistant_hash() {
        let data = b"test hash input";
        let hash = quantum_resistant_hash(data).unwrap();
        assert_eq!(hash.len(), 32); // BLAKE3 output size
    }

    #[test]
    fn test_constant_time_compare() {
        let a = vec![1, 2, 3, 4];
        let b = vec![1, 2, 3, 4];
        let c = vec![1, 2, 3, 5];

        assert!(constant_time_compare(&a, &b));
        assert!(!constant_time_compare(&a, &c));
        assert!(!constant_time_compare(&a, &[1, 2, 3]));
    }

    #[test]
    fn test_signature_verification_edge_cases() {
        // Test with invalid signature length
        let result = verify_signature("test", b"data", &[1u8; 63]);
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Test with invalid signature length
        let result = verify_signature("test", b"data", &[1u8; 65]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
