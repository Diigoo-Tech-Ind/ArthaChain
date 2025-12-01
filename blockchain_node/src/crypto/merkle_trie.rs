//! Merkle Patricia Trie Implementation for State Root Calculation
//! This module provides efficient state root calculation for blockchain state

use anyhow::{anyhow, Result};
use ethereum_types::H256;
use rlp::RlpStream;
use sha3::{Digest, Keccak256};
use std::collections::HashMap;

/// Merkle Patricia Trie node types
#[derive(Debug, Clone)]
enum TrieNode {
    /// Empty node
    Empty,
    /// Leaf node (path, value)
    Leaf(Vec<u8>, Vec<u8>),
    /// Extension node (path, next node hash)
    Extension(Vec<u8>, H256),
    /// Branch node (16 children + optional value)
    Branch([Option<H256>; 16], Option<Vec<u8>>),
}

/// Merkle Patricia Trie for state storage
pub struct MerklePatriciaTrie {
    /// In-memory storage of trie nodes
    nodes: HashMap<H256, TrieNode>,
    /// Root hash of the trie
    root: H256,
}

impl MerklePatriciaTrie {
    /// Create a new empty Merkle Patricia Trie
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: H256::zero(),
        }
    }

    /// Insert a key-value pair into the trie
    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let nibbles = Self::bytes_to_nibbles(key);
        self.root = self.insert_recursive(&nibbles, value, self.root)?;
        Ok(())
    }

    /// Get a value from the trie
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let nibbles = Self::bytes_to_nibbles(key);
        self.get_recursive(&nibbles, self.root)
    }

    /// Get the root hash of the trie
    pub fn root_hash(&self) -> H256 {
        self.root
    }

    /// Calculate state root from a list of (address, account) pairs
    pub fn calculate_state_root(accounts: &[(Vec<u8>, Vec<u8>)]) -> Result<H256> {
        let mut trie = Self::new();
        for (address, account_data) in accounts {
            trie.insert(address, account_data)?;
        }
        Ok(trie.root_hash())
    }

    /// Convert bytes to nibbles (4-bit values)
    fn bytes_to_nibbles(bytes: &[u8]) -> Vec<u8> {
        let mut nibbles = Vec::with_capacity(bytes.len() * 2);
        for byte in bytes {
            nibbles.push(byte >> 4);
            nibbles.push(byte & 0x0F);
        }
        nibbles
    }

    /// Convert nibbles back to bytes
    fn nibbles_to_bytes(nibbles: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(nibbles.len().div_ceil(2));
        for chunk in nibbles.chunks(2) {
            if chunk.len() == 2 {
                bytes.push((chunk[0] << 4) | chunk[1]);
            } else {
                bytes.push(chunk[0] << 4);
            }
        }
        bytes
    }

    /// Insert recursively into the trie
    fn insert_recursive(&mut self, path: &[u8], value: &[u8], node_hash: H256) -> Result<H256> {
        if node_hash == H256::zero() {
            // Create new leaf node
            let leaf = TrieNode::Leaf(path.to_vec(), value.to_vec());
            return self.store_node(leaf);
        }

        let node = self
            .nodes
            .get(&node_hash)
            .ok_or_else(|| anyhow!("Node not found"))?
            .clone();

        match node {
            TrieNode::Leaf(existing_path, existing_value) => {
                let common_prefix = Self::common_prefix(&existing_path, path);

                if common_prefix == existing_path.len() && common_prefix == path.len() {
                    // Same key, update value
                    let updated_leaf = TrieNode::Leaf(path.to_vec(), value.to_vec());
                    self.store_node(updated_leaf)
                } else if common_prefix == existing_path.len() {
                    // Extend the existing leaf
                    let mut branch = [None; 16];
                    let next_nibble = path[common_prefix] as usize;
                    let new_leaf = TrieNode::Leaf(path[common_prefix + 1..].to_vec(), value.to_vec());
                    branch[next_nibble] = Some(self.store_node(new_leaf)?);
                    
                    let branch_node = TrieNode::Branch(branch, Some(existing_value));
                    self.store_node(branch_node)
                } else {
                    // Create branch node
                    let mut branch = [None; 16];
                    
                    if common_prefix < existing_path.len() {
                        let existing_nibble = existing_path[common_prefix] as usize;
                        let existing_leaf = TrieNode::Leaf(
                            existing_path[common_prefix + 1..].to_vec(),
                            existing_value,
                        );
                        branch[existing_nibble] = Some(self.store_node(existing_leaf)?);
                    }
                    
                    if common_prefix < path.len() {
                        let new_nibble = path[common_prefix] as usize;
                        let new_leaf = TrieNode::Leaf(path[common_prefix + 1..].to_vec(), value.to_vec());
                        branch[new_nibble] = Some(self.store_node(new_leaf)?);
                    }
                    
                    let branch_node = if common_prefix == path.len() {
                        TrieNode::Branch(branch, Some(value.to_vec()))
                    } else {
                        TrieNode::Branch(branch, None)
                    };
                    
                    if common_prefix > 0 {
                        let branch_hash = self.store_node(branch_node)?;
                        let extension = TrieNode::Extension(path[..common_prefix].to_vec(), branch_hash);
                        self.store_node(extension)
                    } else {
                        self.store_node(branch_node)
                    }
                }
            }
            TrieNode::Branch(mut children, branch_value) => {
                if path.is_empty() {
                    // Update branch value
                    let updated_branch = TrieNode::Branch(children, Some(value.to_vec()));
                    self.store_node(updated_branch)
                } else {
                    let nibble = path[0] as usize;
                    let child_hash = children[nibble].unwrap_or(H256::zero());
                    let new_child_hash = self.insert_recursive(&path[1..], value, child_hash)?;
                    children[nibble] = Some(new_child_hash);
                    
                    let updated_branch = TrieNode::Branch(children, branch_value);
                    self.store_node(updated_branch)
                }
            }
            TrieNode::Extension(ext_path, next_hash) => {
                let common_prefix = Self::common_prefix(&ext_path, path);
                
                if common_prefix == ext_path.len() {
                    // Follow the extension
                    let new_next_hash = self.insert_recursive(&path[common_prefix..], value, next_hash)?;
                    let updated_ext = TrieNode::Extension(ext_path, new_next_hash);
                    self.store_node(updated_ext)
                } else {
                    // Split the extension
                    let mut branch = [None; 16];
                    
                    // Add the remainder of the extension
                    if common_prefix < ext_path.len() {
                        let ext_nibble = ext_path[common_prefix] as usize;
                        let remaining_ext = TrieNode::Extension(
                            ext_path[common_prefix + 1..].to_vec(),
                            next_hash,
                        );
                        branch[ext_nibble] = Some(self.store_node(remaining_ext)?);
                    }
                    
                    // Add the new path
                    if common_prefix < path.len() {
                        let new_nibble = path[common_prefix] as usize;
                        let new_leaf = TrieNode::Leaf(path[common_prefix + 1..].to_vec(), value.to_vec());
                        branch[new_nibble] = Some(self.store_node(new_leaf)?);
                    }
                    
                    let branch_node = TrieNode::Branch(branch, None);
                    let branch_hash = self.store_node(branch_node)?;
                    
                    if common_prefix > 0 {
                        let new_ext = TrieNode::Extension(path[..common_prefix].to_vec(), branch_hash);
                        self.store_node(new_ext)
                    } else {
                        Ok(branch_hash)
                    }
                }
            }
            TrieNode::Empty => {
                let leaf = TrieNode::Leaf(path.to_vec(), value.to_vec());
                self.store_node(leaf)
            }
        }
    }

    /// Get value recursively from the trie
    fn get_recursive(&self, path: &[u8], node_hash: H256) -> Result<Option<Vec<u8>>> {
        if node_hash == H256::zero() {
            return Ok(None);
        }

        let node = self
            .nodes
            .get(&node_hash)
            .ok_or_else(|| anyhow!("Node not found"))?;

        match node {
            TrieNode::Leaf(leaf_path, value) => {
                if leaf_path == path {
                    Ok(Some(value.clone()))
                } else {
                    Ok(None)
                }
            }
            TrieNode::Branch(children, branch_value) => {
                if path.is_empty() {
                    Ok(branch_value.clone())
                } else {
                    let nibble = path[0] as usize;
                    if let Some(child_hash) = children[nibble] {
                        self.get_recursive(&path[1..], child_hash)
                    } else {
                        Ok(None)
                    }
                }
            }
            TrieNode::Extension(ext_path, next_hash) => {
                if path.starts_with(ext_path) {
                    self.get_recursive(&path[ext_path.len()..], *next_hash)
                } else {
                    Ok(None)
                }
            }
            TrieNode::Empty => Ok(None),
        }
    }

    /// Store a node and return its hash
    fn store_node(&mut self, node: TrieNode) -> Result<H256> {
        let hash = self.hash_node(&node);
        self.nodes.insert(hash, node);
        Ok(hash)
    }

    /// Calculate hash of a node
    fn hash_node(&self, node: &TrieNode) -> H256 {
        let encoded = self.encode_node(node);
        let mut hasher = Keccak256::new();
        hasher.update(&encoded);
        H256::from_slice(&hasher.finalize())
    }

    /// RLP encode a node
    fn encode_node(&self, node: &TrieNode) -> Vec<u8> {
        let mut stream = RlpStream::new();
        match node {
            TrieNode::Empty => {
                stream.append_empty_data();
            }
            TrieNode::Leaf(path, value) => {
                stream.begin_list(2);
                stream.append(&Self::encode_path(path, true));
                stream.append(value);
            }
            TrieNode::Extension(path, next_hash) => {
                stream.begin_list(2);
                stream.append(&Self::encode_path(path, false));
                stream.append(&next_hash.as_bytes().to_vec());
            }
            TrieNode::Branch(children, value) => {
                stream.begin_list(17);
                for child in children.iter() {
                    if let Some(hash) = child {
                        stream.append(&hash.as_bytes().to_vec());
                    } else {
                        stream.append_empty_data();
                    }
                }
                if let Some(v) = value {
                    stream.append(v);
                } else {
                    stream.append_empty_data();
                }
            }
        }
        stream.out().to_vec()
    }

    /// Encode path with terminator flag
    fn encode_path(nibbles: &[u8], is_leaf: bool) -> Vec<u8> {
        let mut encoded = Vec::new();
        let terminator = if is_leaf { 0x20 } else { 0x00 };
        
        if nibbles.len() % 2 == 0 {
            encoded.push(terminator);
            encoded.extend(Self::nibbles_to_bytes(nibbles));
        } else {
            encoded.push(terminator | 0x10 | nibbles[0]);
            encoded.extend(Self::nibbles_to_bytes(&nibbles[1..]));
        }
        
        encoded
    }

    /// Find common prefix length between two paths
    fn common_prefix(a: &[u8], b: &[u8]) -> usize {
        let min_len = a.len().min(b.len());
        for i in 0..min_len {
            if a[i] != b[i] {
                return i;
            }
        }
        min_len
    }
}

impl Default for MerklePatriciaTrie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_trie_insert_get() {
        let mut trie = MerklePatriciaTrie::new();
        
        let key = b"test_key";
        let value = b"test_value";
        
        trie.insert(key, value).unwrap();
        let retrieved = trie.get(key).unwrap();
        
        assert_eq!(retrieved, Some(value.to_vec()));
    }

    #[test]
    fn test_state_root_calculation() {
        let accounts = vec![
            (b"address1".to_vec(), b"account_data1".to_vec()),
            (b"address2".to_vec(), b"account_data2".to_vec()),
        ];
        
        let root = MerklePatriciaTrie::calculate_state_root(&accounts).unwrap();
        assert_ne!(root, H256::zero());
    }
}
