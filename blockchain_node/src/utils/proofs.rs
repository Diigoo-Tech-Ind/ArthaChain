use crate::utils::crypto::Hash;
use std::result::Result;
use sha2::{Sha256, Digest};

/// Merkle proof structure
#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub leaf_hash: Hash,
    pub path: Vec<Hash>,
    pub root_hash: Hash,
}

/// Verify a merkle proof
pub fn verify_proof(
    prev_root: &Hash,
    new_root: &Hash,
    proof: &[u8],
) -> Result<bool, Box<dyn std::error::Error>> {
    // Parse the proof bytes
    let merkle_proof = parse_merkle_proof(proof)?;
    
    // Verify the proof
    let computed_root = compute_merkle_root(&merkle_proof)?;
    
    // Check if the computed root matches the expected root
    Ok(computed_root == *new_root && *prev_root != *new_root)
}

/// Parse merkle proof from bytes
fn parse_merkle_proof(proof: &[u8]) -> Result<MerkleProof, Box<dyn std::error::Error>> {
    if proof.len() < 8 {
        return Err("Invalid proof format".into());
    }
    
    let mut offset = 0;
    
    // Read leaf index (8 bytes)
    let leaf_index = u64::from_le_bytes([
        proof[offset], proof[offset + 1], proof[offset + 2], proof[offset + 3],
        proof[offset + 4], proof[offset + 5], proof[offset + 6], proof[offset + 7],
    ]) as usize;
    offset += 8;
    
    // Read leaf hash (32 bytes)
    if proof.len() < offset + 32 {
        return Err("Invalid proof format".into());
    }
    let leaf_hash = Hash::new([
        proof[offset], proof[offset + 1], proof[offset + 2], proof[offset + 3],
        proof[offset + 4], proof[offset + 5], proof[offset + 6], proof[offset + 7],
        proof[offset + 8], proof[offset + 9], proof[offset + 10], proof[offset + 11],
        proof[offset + 12], proof[offset + 13], proof[offset + 14], proof[offset + 15],
        proof[offset + 16], proof[offset + 17], proof[offset + 18], proof[offset + 19],
        proof[offset + 20], proof[offset + 21], proof[offset + 22], proof[offset + 23],
        proof[offset + 24], proof[offset + 25], proof[offset + 26], proof[offset + 27],
        proof[offset + 28], proof[offset + 29], proof[offset + 30], proof[offset + 31],
    ]);
    offset += 32;
    
    // Read path length (4 bytes)
    if proof.len() < offset + 4 {
        return Err("Invalid proof format".into());
    }
    let path_len = u32::from_le_bytes([
        proof[offset], proof[offset + 1], proof[offset + 2], proof[offset + 3],
    ]) as usize;
    offset += 4;
    
    // Read path hashes
    let mut path = Vec::new();
    for _ in 0..path_len {
        if proof.len() < offset + 32 {
            return Err("Invalid proof format".into());
        }
        let hash = Hash::new([
            proof[offset], proof[offset + 1], proof[offset + 2], proof[offset + 3],
            proof[offset + 4], proof[offset + 5], proof[offset + 6], proof[offset + 7],
            proof[offset + 8], proof[offset + 9], proof[offset + 10], proof[offset + 11],
            proof[offset + 12], proof[offset + 13], proof[offset + 14], proof[offset + 15],
            proof[offset + 16], proof[offset + 17], proof[offset + 18], proof[offset + 19],
            proof[offset + 20], proof[offset + 21], proof[offset + 22], proof[offset + 23],
            proof[offset + 24], proof[offset + 25], proof[offset + 26], proof[offset + 27],
            proof[offset + 28], proof[offset + 29], proof[offset + 30], proof[offset + 31],
        ]);
        path.push(hash);
        offset += 32;
    }
    
    // Read root hash (32 bytes)
    if proof.len() < offset + 32 {
        return Err("Invalid proof format".into());
    }
    let root_hash = Hash::new([
        proof[offset], proof[offset + 1], proof[offset + 2], proof[offset + 3],
        proof[offset + 4], proof[offset + 5], proof[offset + 6], proof[offset + 7],
        proof[offset + 8], proof[offset + 9], proof[offset + 10], proof[offset + 11],
        proof[offset + 12], proof[offset + 13], proof[offset + 14], proof[offset + 15],
        proof[offset + 16], proof[offset + 17], proof[offset + 18], proof[offset + 19],
        proof[offset + 20], proof[offset + 21], proof[offset + 22], proof[offset + 23],
        proof[offset + 24], proof[offset + 25], proof[offset + 26], proof[offset + 27],
        proof[offset + 28], proof[offset + 29], proof[offset + 30], proof[offset + 31],
    ]);
    
    Ok(MerkleProof {
        leaf_index,
        leaf_hash,
        path,
        root_hash,
    })
}

/// Compute merkle root from proof
fn compute_merkle_root(proof: &MerkleProof) -> Result<Hash, Box<dyn std::error::Error>> {
    let mut current_hash = proof.leaf_hash.clone();
    let mut index = proof.leaf_index;
    
    for sibling_hash in &proof.path {
        let mut hasher = Sha256::new();
        
        if index % 2 == 0 {
            // Current node is left child
            hasher.update(current_hash.as_bytes());
            hasher.update(sibling_hash.as_bytes());
        } else {
            // Current node is right child
            hasher.update(sibling_hash.as_bytes());
            hasher.update(current_hash.as_bytes());
        }
        
        let hash_bytes = hasher.finalize();
        current_hash = Hash::new(hash_bytes.into());
        index /= 2;
    }
    
    Ok(current_hash)
}
