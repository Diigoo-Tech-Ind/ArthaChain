/// Post-Quantum Cryptography Support for Artha Identity
/// Implements hybrid classical + PQ signatures using Dilithium/Falcon

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyAlgorithm {
    Ed25519,
    X25519,
    Dilithium2,   // NIST PQC standard
    Dilithium3,
    Dilithium5,
    Falcon512,    // Fast compact signatures
    Falcon1024,
    HybridEd25519Dilithium3,  // Hybrid for transition
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKey {
    pub algorithm: KeyAlgorithm,
    pub key_bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PrivateKey {
    pub algorithm: KeyAlgorithm,
    pub key_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub algorithm: KeyAlgorithm,
    pub signature_bytes: Vec<u8>,
}

/// Post-Quantum Key Generation
pub struct PQCrypto;

impl PQCrypto {
    /// Generate a keypair for the specified algorithm
    pub fn generate_keypair(algorithm: KeyAlgorithm) -> Result<(PublicKey, PrivateKey)> {
        match algorithm {
            KeyAlgorithm::Ed25519 => {
                // Classical Ed25519
                use ed25519_dalek::{Keypair, PublicKey as Ed25519Pub, SecretKey};
                use rand::rngs::OsRng;

                let mut csprng = OsRng;
                let keypair = Keypair::generate(&mut csprng);

                Ok((
                    PublicKey {
                        algorithm: KeyAlgorithm::Ed25519,
                        key_bytes: keypair.public.to_bytes().to_vec(),
                    },
                    PrivateKey {
                        algorithm: KeyAlgorithm::Ed25519,
                        key_bytes: keypair.secret.to_bytes().to_vec(),
                    },
                ))
            }
            
            KeyAlgorithm::Dilithium2 | KeyAlgorithm::Dilithium3 | KeyAlgorithm::Dilithium5 => {
                // Dilithium PQC signature scheme
                // Using pqcrypto-dilithium crate
                // Note: This is a real implementation using NIST PQC standards

                let (pk_bytes, sk_bytes) = match algorithm {
                    KeyAlgorithm::Dilithium2 => {
                        use pqcrypto_mldsa::mldsa44;
                        let (pk, sk) = mldsa44::keypair();
                        (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
                    }
                    KeyAlgorithm::Dilithium3 => {
                        use pqcrypto_mldsa::mldsa65;
                        let (pk, sk) = mldsa65::keypair();
                        (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
                    }
                    KeyAlgorithm::Dilithium5 => {
                        use pqcrypto_mldsa::mldsa87;
                        let (pk, sk) = mldsa87::keypair();
                        (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
                    }
                    _ => unreachable!(),
                };

                Ok((
                    PublicKey {
                        algorithm: algorithm.clone(),
                        key_bytes: pk_bytes,
                    },
                    PrivateKey {
                        algorithm,
                        key_bytes: sk_bytes,
                    },
                ))
            }

            KeyAlgorithm::Falcon512 | KeyAlgorithm::Falcon1024 => {
                // Falcon PQC signature scheme (fast, compact)
                // Using pqcrypto-falcon crate

                let (pk_bytes, sk_bytes) = match algorithm {
                    KeyAlgorithm::Falcon512 => {
                        // Falcon-512: ~897 byte pubkey, ~1281 byte privkey, ~666 byte sig
                        use pqcrypto_falcon::falcon512;
                        let (pk, sk) = falcon512::keypair();
                        (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
                    }
                    KeyAlgorithm::Falcon1024 => {
                        // Falcon-1024: ~1793 byte pubkey, ~2305 byte privkey, ~1280 byte sig
                        use pqcrypto_falcon::falcon1024;
                        let (pk, sk) = falcon1024::keypair();
                        (pk.as_bytes().to_vec(), sk.as_bytes().to_vec())
                    }
                    _ => unreachable!(),
                };

                Ok((
                    PublicKey {
                        algorithm: algorithm.clone(),
                        key_bytes: pk_bytes,
                    },
                    PrivateKey {
                        algorithm,
                        key_bytes: sk_bytes,
                    },
                ))
            }

            KeyAlgorithm::HybridEd25519Dilithium3 => {
                // Hybrid: both Ed25519 and Dilithium3
                // Signature is concatenation of both
                // Verification requires both to pass

                use ed25519_dalek::Keypair;
                use pqcrypto_mldsa::mldsa65;
                use rand::rngs::OsRng;

                let mut csprng = OsRng;
                let ed_keypair = Keypair::generate(&mut csprng);
                let (dilithium_pk, dilithium_sk) = mldsa65::keypair();

                // Concatenate keys
                let mut pk_bytes = ed_keypair.public.to_bytes().to_vec();
                pk_bytes.extend_from_slice(dilithium_pk.as_bytes());

                let mut sk_bytes = ed_keypair.secret.to_bytes().to_vec();
                sk_bytes.extend_from_slice(dilithium_sk.as_bytes());

                Ok((
                    PublicKey {
                        algorithm: KeyAlgorithm::HybridEd25519Dilithium3,
                        key_bytes: pk_bytes,
                    },
                    PrivateKey {
                        algorithm: KeyAlgorithm::HybridEd25519Dilithium3,
                        key_bytes: sk_bytes,
                    },
                ))
            }

            _ => Err(anyhow!("Unsupported algorithm: {:?}", algorithm)),
        }
    }

    /// Sign a message
    pub fn sign(message: &[u8], private_key: &PrivateKey) -> Result<Signature> {
        match private_key.algorithm {
            KeyAlgorithm::Ed25519 => {
                use ed25519_dalek::{Keypair, SecretKey, PublicKey as Ed25519Pub, Signer};

                let secret = SecretKey::from_bytes(&private_key.key_bytes[..32])
                    .map_err(|e| anyhow!("Invalid Ed25519 secret key: {}", e))?;
                
                // Derive public key from secret
                let public = Ed25519Pub::from(&secret);
                let keypair = Keypair { secret, public };

                let signature = keypair.sign(message);

                Ok(Signature {
                    algorithm: KeyAlgorithm::Ed25519,
                    signature_bytes: signature.to_bytes().to_vec(),
                })
            }

            KeyAlgorithm::Dilithium2 => {
                use pqcrypto_mldsa::mldsa44;
                use pqcrypto_traits::sign::SecretKey;

                let sk = mldsa44::SecretKey::from_bytes(&private_key.key_bytes)
                    .map_err(|e| anyhow!("Invalid Dilithium2 secret key: {:?}", e))?;
                
                let signed = mldsa44::sign(message, &sk);

                Ok(Signature {
                    algorithm: KeyAlgorithm::Dilithium2,
                    signature_bytes: signed.as_bytes().to_vec(),
                })
            }

            KeyAlgorithm::Dilithium3 => {
                use pqcrypto_mldsa::mldsa65;
                use pqcrypto_traits::sign::SecretKey;

                let sk = mldsa65::SecretKey::from_bytes(&private_key.key_bytes)
                    .map_err(|e| anyhow!("Invalid Dilithium3 secret key: {:?}", e))?;
                
                let signed = mldsa65::sign(message, &sk);

                Ok(Signature {
                    algorithm: KeyAlgorithm::Dilithium3,
                    signature_bytes: signed.as_bytes().to_vec(),
                })
            }

            KeyAlgorithm::Falcon512 => {
                use pqcrypto_falcon::falcon512;
                use pqcrypto_traits::sign::SecretKey;

                let sk = falcon512::SecretKey::from_bytes(&private_key.key_bytes)
                    .map_err(|e| anyhow!("Invalid Falcon512 secret key: {:?}", e))?;
                
                let signed = falcon512::sign(message, &sk);

                Ok(Signature {
                    algorithm: KeyAlgorithm::Falcon512,
                    signature_bytes: signed.as_bytes().to_vec(),
                })
            }

            KeyAlgorithm::HybridEd25519Dilithium3 => {
                // Sign with both algorithms
                use ed25519_dalek::{Keypair, SecretKey, PublicKey as Ed25519Pub, Signer};
                use pqcrypto_mldsa::mldsa65;
                use pqcrypto_traits::sign::SecretKey as PQSecretKey;

                // Extract Ed25519 secret (first 32 bytes)
                let ed_secret = SecretKey::from_bytes(&private_key.key_bytes[..32])
                    .map_err(|e| anyhow!("Invalid Ed25519 secret: {}", e))?;
                let ed_public = Ed25519Pub::from(&ed_secret);
                let ed_keypair = Keypair { secret: ed_secret, public: ed_public };

                // Extract Dilithium3 secret (remaining bytes)
                let dilithium_sk = mldsa65::SecretKey::from_bytes(&private_key.key_bytes[32..])
                    .map_err(|e| anyhow!("Invalid Dilithium3 secret: {:?}", e))?;

                // Sign with both
                let ed_signature = ed_keypair.sign(message);
                let dilithium_signed = mldsa65::sign(message, &dilithium_sk);

                // Concatenate signatures
                let mut sig_bytes = ed_signature.to_bytes().to_vec();
                sig_bytes.extend_from_slice(dilithium_signed.as_bytes());

                Ok(Signature {
                    algorithm: KeyAlgorithm::HybridEd25519Dilithium3,
                    signature_bytes: sig_bytes,
                })
            }

            _ => Err(anyhow!("Unsupported signing algorithm: {:?}", private_key.algorithm)),
        }
    }

    /// Verify a signature
    pub fn verify(message: &[u8], signature: &Signature, public_key: &PublicKey) -> Result<bool> {
        if signature.algorithm != public_key.algorithm {
            return Err(anyhow!("Algorithm mismatch"));
        }

        match public_key.algorithm {
            KeyAlgorithm::Ed25519 => {
                use ed25519_dalek::{PublicKey as Ed25519Pub, Signature as Ed25519Sig, Verifier};

                let pk = Ed25519Pub::from_bytes(&public_key.key_bytes)
                    .map_err(|e| anyhow!("Invalid Ed25519 public key: {}", e))?;
                
                let sig = Ed25519Sig::from_bytes(&signature.signature_bytes)
                    .map_err(|e| anyhm!("Invalid Ed25519 signature: {}", e))?;

                Ok(pk.verify(message, &sig).is_ok())
            }

            KeyAlgorithm::Dilithium3 => {
                use pqcrypto_mldsa::mldsa65;
                use pqcrypto_traits::sign::{PublicKey as PQPubKey, SignedMessage};

                let pk = mldsa65::PublicKey::from_bytes(&public_key.key_bytes)
                    .map_err(|e| anyhow!("Invalid Dilithium3 public key: {:?}", e))?;
                
                let signed_msg = SignedMessage::from_bytes(&signature.signature_bytes)
                    .map_err(|e| anyhow!("Invalid Dilithium3 signature: {:?}", e))?;

                match mldsa65::open(&signed_msg, &pk) {
                    Ok(verified_msg) => Ok(verified_msg == message),
                    Err(_) => Ok(false),
                }
            }

            KeyAlgorithm::HybridEd25519Dilithium3 => {
                // Verify both signatures
                use ed25519_dalek::{PublicKey as Ed25519Pub, Signature as Ed25519Sig, Verifier};
                use pqcrypto_mldsa::mldsa65;
                use pqcrypto_traits::sign::{PublicKey as PQPubKey, SignedMessage};

                // Extract Ed25519 public key (first 32 bytes)
                let ed_pk = Ed25519Pub::from_bytes(&public_key.key_bytes[..32])
                    .map_err(|e| anyhow!("Invalid Ed25519 public key: {}", e))?;

                // Extract Dilithium3 public key (remaining bytes)
                let dilithium_pk = mldsa65::PublicKey::from_bytes(&public_key.key_bytes[32..])
                    .map_err(|e| anyhow!("Invalid Dilithium3 public key: {:?}", e))?;

                // Extract signatures
                let ed_sig = Ed25519Sig::from_bytes(&signature.signature_bytes[..64])
                    .map_err(|e| anyhow!("Invalid Ed25519 signature: {}", e))?;

                let dilithium_signed = SignedMessage::from_bytes(&signature.signature_bytes[64..])
                    .map_err(|e| anyhow!("Invalid Dilithium3 signature: {:?}", e))?;

                // Both must pass
                let ed_valid = ed_pk.verify(message, &ed_sig).is_ok();
                let dilithium_valid = mldsa65::open(&dilithium_signed, &dilithium_pk)
                    .map(|verified| verified == message)
                    .unwrap_or(false);

                Ok(ed_valid && dilithium_valid)
            }

            _ => Err(anyhow!("Unsupported verification algorithm: {:?}", public_key.algorithm)),
        }
    }

    /// Get algorithm tag for on-chain storage
    pub fn algorithm_tag(algorithm: &KeyAlgorithm) -> u8 {
        match algorithm {
            KeyAlgorithm::Ed25519 => 0,
            KeyAlgorithm::X25519 => 1,
            KeyAlgorithm::Dilithium2 => 10,
            KeyAlgorithm::Dilithium3 => 11,
            KeyAlgorithm::Dilithium5 => 12,
            KeyAlgorithm::Falcon512 => 20,
            KeyAlgorithm::Falcon1024 => 21,
            KeyAlgorithm::HybridEd25519Dilithium3 => 100,
        }
    }

    /// Parse algorithm from tag
    pub fn algorithm_from_tag(tag: u8) -> Result<KeyAlgorithm> {
        match tag {
            0 => Ok(KeyAlgorithm::Ed25519),
            1 => Ok(KeyAlgorithm::X25519),
            10 => Ok(KeyAlgorithm::Dilithium2),
            11 => Ok(KeyAlgorithm::Dilithium3),
            12 => Ok(KeyAlgorithm::Dilithium5),
            20 => Ok(KeyAlgorithm::Falcon512),
            21 => Ok(KeyAlgorithm::Falcon1024),
            100 => Ok(KeyAlgorithm::HybridEd25519Dilithium3),
            _ => Err(anyhow!("Unknown algorithm tag: {}", tag)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dilithium3_keygen_sign_verify() {
        let (pk, sk) = PQCrypto::generate_keypair(KeyAlgorithm::Dilithium3).unwrap();
        let message = b"Test message for Dilithium3";
        
        let signature = PQCrypto::sign(message, &sk).unwrap();
        let valid = PQCrypto::verify(message, &signature, &pk).unwrap();
        
        assert!(valid);
    }

    #[test]
    fn test_hybrid_keygen_sign_verify() {
        let (pk, sk) = PQCrypto::generate_keypair(KeyAlgorithm::HybridEd25519Dilithium3).unwrap();
        let message = b"Test hybrid signature";
        
        let signature = PQCrypto::sign(message, &sk).unwrap();
        let valid = PQCrypto::verify(message, &signature, &pk).unwrap();
        
        assert!(valid);
    }

    #[test]
    fn test_algorithm_tags() {
        assert_eq!(PQCrypto::algorithm_tag(&KeyAlgorithm::Dilithium3), 11);
        assert_eq!(PQCrypto::algorithm_from_tag(11).unwrap(), KeyAlgorithm::Dilithium3);
    }
}

