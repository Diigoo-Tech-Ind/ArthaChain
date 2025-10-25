#[cfg(feature = "erasure")]
mod tests {
    use reed_solomon_erasure::galois_8::ReedSolomon;
    use rand::{RngCore, SeedableRng};
    use rand::rngs::StdRng;

    #[test]
    fn rs_decode_10k_stripes_drop_upto_2() {
        let k = 8usize; let m = 2usize; let n = k + m;
        let rs = ReedSolomon::new(k, m).expect("rs");
        let mut rng = StdRng::seed_from_u64(0x1234_5678_9abc_def0);
        let stripes = 10_000;
        let shard_len = 1024; // 1 KiB shards for test

        for _ in 0..stripes {
            // Build data shards
            let mut shards: Vec<Vec<u8>> = (0..n).map(|_| vec![0u8; shard_len]).collect();
            for i in 0..k {
                rng.fill_bytes(&mut shards[i]);
            }
            // Encode parity
            {
                let mut refs: Vec<&mut [u8]> = shards.iter_mut().map(|v| v.as_mut_slice()).collect();
                rs.encode(&mut refs).expect("encode");
            }
            // Make a copy of original data to verify later
            let original: Vec<Vec<u8>> = shards[..k].iter().cloned().collect();
            // Randomly drop up to 2 shards
            let drop = (rng.next_u32() % 3) as usize; // 0,1,2
            let mut present: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
            for _ in 0..drop {
                let mut idx = (rng.next_u32() as usize) % n;
                // ensure we drop distinct indices
                while present[idx].is_none() { idx = (idx + 1) % n; }
                present[idx] = None;
            }
            // Prepare references for decode
            let mut refs: Vec<Option<&mut [u8]>> = present.iter_mut().map(|opt| opt.as_mut().map(|v| v.as_mut_slice())).collect();
            rs.reconstruct(&mut refs).expect("reconstruct");
            // Verify data shards match original
            for i in 0..k {
                let recovered = refs[i].as_ref().unwrap();
                assert_eq!(recovered, &original[i][..], "mismatch at data shard {}", i);
            }
        }
    }
}


