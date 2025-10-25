use std::fs;
use std::path::PathBuf;

use blake3;

use crate::storage::{
    svdb_storage::SvdbStorage, ChunkStore, Cid, Codec, Manifests, Manifest, ManifestChunkEntry,
    StorageConfig, StorageInit,
};

fn temp_data_dir(prefix: &str) -> String {
    let mut p = std::env::temp_dir();
    let rand = format!("{}_{}", prefix, rand::random::<u64>());
    p.push(rand);
    fs::create_dir_all(&p).unwrap();
    p.to_string_lossy().to_string()
}

#[tokio::test]
async fn test_chunk_put_get_and_manifest_roundtrip() {
    let dir = temp_data_dir("svdb_test");
    let storage = SvdbStorage::default();
    storage
        .init(&StorageConfig {
            data_dir: dir.clone(),
            ..Default::default()
        })
        .await
        .expect("init");

    // Put a chunk
    let data = b"hello world - svdb".to_vec();
    let blake = *blake3::hash(&data).as_bytes();
    let cid = Cid::new(0x0129, blake, None, data.len() as u64, Codec::Raw);
    storage.put(&cid, &data).await.expect("put");
    let got = storage.get(&cid).await.expect("get");
    assert_eq!(got, data);

    // Put a manifest that references the chunk
    let manifest = Manifest {
        version: 1,
        size: data.len() as u64,
        chunks: vec![ManifestChunkEntry { cid: cid.clone(), order: 0 }],
        license: None,
        codec: Codec::Raw,
        erasure_data_shards: Some(8),
        erasure_parity_shards: Some(2),
        merkle_root: blake, // for a single leaf, root == leaf (using our simple test assumption)
        poseidon_root: None,
        envelope: None,
    };
    let m_cid = storage.put_manifest(&manifest).await.expect("put manifest");
    let round = storage.get_manifest(&m_cid).await.expect("get manifest");
    assert_eq!(round.size, manifest.size);
    assert_eq!(round.chunks.len(), 1);
}

#[tokio::test]
async fn test_rs_reconstruct_10_8() {
    let storage = SvdbStorage::default();
    let mut shards: Vec<Option<Vec<u8>>> = Vec::new();
    // Build 10 shards with small data pattern; drop two
    for i in 0..10u8 { shards.push(Some(vec![i; 1024])); }
    // Simulate two missing shards
    shards[2] = None;
    shards[9] = None;

    let mut clone = shards.clone();
    // We don't need init/db for reconstruct helper
    storage.rs_reconstruct_10_8(&mut clone).await.expect("reconstruct");
    // Ensure the missing shards are filled and sized as originals
    assert!(clone[2].as_ref().unwrap().len() == 1024);
    assert!(clone[9].as_ref().unwrap().len() == 1024);
}


