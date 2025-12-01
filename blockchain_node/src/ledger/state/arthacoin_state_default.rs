impl Default for ArthaCoinState {
    fn default() -> Self {
        // This is only for serde default
        // Real initialization should use ArthaCoinState::new()
        let arthacoin = Arc::new(ArthaCoinNative::default());
        let balance_bridge = Arc::new(BalanceBridge::new(arthacoin.clone()));
        Self {
            arthacoin,
            balance_bridge,
            nonces: RwLock::new(HashMap::new()),
            storage: RwLock::new(HashMap::new()),
            height: RwLock::new(0),
            shard_id: 0,
            pending_transactions: RwLock::new(VecDeque::new()),
            tx_history: RwLock::new(HashMap::new()),
            blocks: RwLock::new(HashMap::new()),
            blocks_by_hash: RwLock::new(HashMap::new()),
            latest_block_hash: RwLock::new(
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            ),
            rocksdb: RocksDbStorage::new(),
        }
    }
}
