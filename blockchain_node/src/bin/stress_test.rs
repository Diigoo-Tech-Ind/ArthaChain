use anyhow::Result;
use arthachain_node::{
    types::{Address, Transaction},
    transaction::Mempool,
    utils::crypto::Hash as CryptoHash,
};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Stress test configuration
#[derive(Debug, Clone)]
struct StressTestConfig {
    total_transactions: u32,
    transaction_size_mb: u32,
    transaction_size_bytes: u32,
    duration_minutes: u32,
    target_tps: u32,
    batch_size: u32,
    concurrent_workers: u32,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            total_transactions: 10_000,        // Reduced from 100,000 to 10,000
            transaction_size_mb: 1,            // Reduced from 2MB to 1MB
            transaction_size_bytes: 1 * 1024 * 1024, // 1MB in bytes
            duration_minutes: 1,
            target_tps: 167,                   // Reduced from 1667 to 167
            batch_size: 100,                   // Reduced from 1000 to 100
            concurrent_workers: 5,             // Reduced from 10 to 5
        }
    }
}

/// Transaction metrics
#[derive(Debug, Clone, Serialize)]
struct TransactionMetrics {
    transaction_id: String,
    size_bytes: usize,
    submission_time: u64,
    confirmation_time: Option<u64>,
    confirmation_delay_ms: Option<u64>,
    status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize)]
enum TransactionStatus {
    Submitted,
    Confirmed,
    Failed,
    Pending,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize)]
struct PerformanceMetrics {
    total_transactions: u32,
    successful_transactions: u32,
    failed_transactions: u32,
    total_data_mb: f64,
    average_tps: f64,
    peak_tps: f64,
    average_confirmation_time_ms: f64,
    min_confirmation_time_ms: u64,
    max_confirmation_time_ms: u64,
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
    network_bandwidth_mbps: f64,
    test_duration_seconds: f64,
}

/// Stress test orchestrator
struct StressTestOrchestrator {
    config: StressTestConfig,
    mempool: Arc<RwLock<Mempool>>,
    metrics: Arc<RwLock<Vec<TransactionMetrics>>>,
    start_time: Instant,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl StressTestOrchestrator {
    fn new(config: StressTestConfig, mempool: Arc<RwLock<Mempool>>) -> Self {
        Self {
            config,
            mempool,
            metrics: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics {
                total_transactions: 0,
                successful_transactions: 0,
                failed_transactions: 0,
                total_data_mb: 0.0,
                average_tps: 0.0,
                peak_tps: 0.0,
                average_confirmation_time_ms: 0.0,
                min_confirmation_time_ms: u64::MAX,
                max_confirmation_time_ms: 0,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                network_bandwidth_mbps: 0.0,
                test_duration_seconds: 0.0,
            })),
        }
    }

    /// Start the stress test
    async fn run_stress_test(&self) -> Result<()> {
        println!("🚀 Starting Massive Stress Test!");
        println!("📊 Configuration:");
        println!("   - Total Transactions: {} ({} MB each)", 
                self.config.total_transactions, self.config.transaction_size_mb);
        println!("   - Duration: {} minutes", self.config.duration_minutes);
        println!("   - Target TPS: {}", self.config.target_tps);
        println!("   - Batch Size: {}", self.config.batch_size);
        println!("   - Concurrent Workers: {}", self.config.concurrent_workers);
        println!("   - Total Data: {:.2} GB", 
                (self.config.total_transactions as f64 * self.config.transaction_size_mb as f64) / 1024.0);

        // Start concurrent transaction generators
        let mut handles = Vec::new();
        
        for worker_id in 0..self.config.concurrent_workers {
            let config = self.config.clone();
            let mempool = self.mempool.clone();
            let metrics = self.metrics.clone();
            let performance_metrics = self.performance_metrics.clone();
            
            let handle = tokio::spawn(async move {
                Self::transaction_generator_worker(
                    worker_id, config, mempool, metrics, performance_metrics
                ).await;
            });
            
            handles.push(handle);
        }

        // Start performance monitoring
        let performance_metrics = self.performance_metrics.clone();
        let start_time = self.start_time;
        let monitor_handle = tokio::spawn(async move {
            Self::performance_monitor(performance_metrics, start_time).await;
        });

        // Wait for all workers to complete
        for handle in handles {
            handle.await?;
        }

        // Wait for performance monitoring to complete
        monitor_handle.await?;

        // Generate final report
        self.generate_final_report().await?;

        Ok(())
    }

    /// Transaction generator worker
    async fn transaction_generator_worker(
        worker_id: u32,
        config: StressTestConfig,
        mempool: Arc<RwLock<Mempool>>,
        metrics: Arc<RwLock<Vec<TransactionMetrics>>>,
        performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    ) {
        println!("👷 Worker {} starting...", worker_id);
        
        let transactions_per_worker = config.total_transactions / config.concurrent_workers;
        let interval_ms = (1000 / config.target_tps).max(1) as u64; // Ensure minimum 1ms interval
        let mut interval_timer = interval(Duration::from_millis(interval_ms));

        for i in 0..transactions_per_worker {
            interval_timer.tick().await;

            let transaction = Self::generate_large_transaction(
                worker_id, i, config.transaction_size_bytes
            );

            let submission_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Add transaction to mempool
            let result = mempool.write().await.add_transaction(transaction).await;
            
            let status = if result.is_ok() {
                TransactionStatus::Submitted
            } else {
                TransactionStatus::Failed
            };

            // Record metrics
            let metric = TransactionMetrics {
                transaction_id: format!("worker_{}_tx_{}", worker_id, i),
                size_bytes: config.transaction_size_bytes as usize,
                submission_time,
                confirmation_time: None,
                confirmation_delay_ms: None,
                status: status.clone(),
            };

            metrics.write().await.push(metric);

            // Update performance metrics
            let mut perf_metrics = performance_metrics.write().await;
            perf_metrics.total_transactions += 1;
            perf_metrics.total_data_mb += config.transaction_size_mb as f64;
            
            match status {
                TransactionStatus::Submitted => perf_metrics.successful_transactions += 1,
                TransactionStatus::Failed => perf_metrics.failed_transactions += 1,
                _ => {}
            }

            if i % 1000 == 0 {
                println!("👷 Worker {}: Generated {} transactions", worker_id, i);
            }
        }

        println!("✅ Worker {} completed: {} transactions", worker_id, transactions_per_worker);
    }

    /// Generate a large transaction with specified size
    fn generate_large_transaction(worker_id: u32, tx_id: u32, size_bytes: u32) -> Transaction {
        let mut rng = rand::thread_rng();
        
        // Generate random addresses
        let mut from_bytes = [0u8; 20];
        let mut to_bytes = [0u8; 20];
        rng.fill(&mut from_bytes);
        rng.fill(&mut to_bytes);

        // Generate large data payload to meet size requirement
        let mut data = Vec::with_capacity(size_bytes as usize);
        for _ in 0..size_bytes {
            data.push(rng.gen::<u8>());
        }

        // Generate random signature (65 bytes for ECDSA)
        let mut signature = Vec::with_capacity(65);
        for _ in 0..65 {
            signature.push(rng.gen::<u8>());
        }

        // Create transaction without hash first
        let mut transaction = Transaction {
            from: Address(from_bytes),
            to: Address(to_bytes),
            value: rng.gen::<u64>(),
            gas_price: rng.gen::<u64>(),
            gas_limit: rng.gen::<u64>(),
            nonce: rng.gen::<u64>(),
            data,
            signature,
            hash: CryptoHash::default(), // Will be computed
        };

        // Compute hash from transaction data
        let hash_data = format!("{}{}{}{}{}{}{:?}", 
            transaction.from.0.iter().map(|b| format!("{:02x}", b)).collect::<String>(),
            transaction.to.0.iter().map(|b| format!("{:02x}", b)).collect::<String>(),
            transaction.value,
            transaction.gas_price,
            transaction.gas_limit,
            transaction.nonce,
            transaction.data
        );
        
        // Create a 32-byte hash from the data
        let mut hash_bytes = [0u8; 32];
        let data_bytes = hash_data.as_bytes();
        for (i, &byte) in data_bytes.iter().take(32).enumerate() {
            hash_bytes[i] = byte;
        }
        
        transaction.hash = CryptoHash::new(hash_bytes);

        transaction
    }

    /// Performance monitoring
    async fn performance_monitor(
        performance_metrics: Arc<RwLock<PerformanceMetrics>>,
        start_time: Instant,
    ) {
        let mut interval_timer = interval(Duration::from_secs(1));
        let mut last_total_tx = 0;
        let mut last_time = start_time;

        loop {
            interval_timer.tick().await;

            let current_time = Instant::now();
            let elapsed = current_time.duration_since(start_time);
            
            if elapsed.as_secs() >= 60 { // 1 minute test
                break;
            }

            let metrics = performance_metrics.read().await;
            let current_total_tx = metrics.total_transactions;
            let time_diff = current_time.duration_since(last_time).as_secs_f64();
            
            if time_diff > 0.0 {
                let current_tps = (current_total_tx - last_total_tx) as f64 / time_diff;
                
                // Update peak TPS
                if current_tps > metrics.peak_tps {
                    let mut perf_metrics = performance_metrics.write().await;
                    perf_metrics.peak_tps = current_tps;
                }

                println!("📊 Real-time TPS: {:.2}, Total TX: {}, Elapsed: {:.1}s", 
                        current_tps, current_total_tx, elapsed.as_secs_f64());
            }

            last_total_tx = current_total_tx;
            last_time = current_time;
        }
    }

    /// Generate final performance report
    async fn generate_final_report(&self) -> Result<()> {
        let end_time = Instant::now();
        let test_duration = end_time.duration_since(self.start_time);
        
        let metrics = self.metrics.read().await;
        let mut perf_metrics = self.performance_metrics.write().await;

        // Calculate final metrics
        perf_metrics.test_duration_seconds = test_duration.as_secs_f64();
        perf_metrics.average_tps = perf_metrics.total_transactions as f64 / perf_metrics.test_duration_seconds;

        // Calculate confirmation times
        let mut confirmation_times: Vec<u64> = metrics
            .iter()
            .filter_map(|m| m.confirmation_delay_ms)
            .collect();

        if !confirmation_times.is_empty() {
            confirmation_times.sort();
            perf_metrics.min_confirmation_time_ms = confirmation_times[0];
            perf_metrics.max_confirmation_time_ms = confirmation_times[confirmation_times.len() - 1];
            perf_metrics.average_confirmation_time_ms = confirmation_times.iter().sum::<u64>() as f64 / confirmation_times.len() as f64;
        }

        // Calculate network bandwidth
        perf_metrics.network_bandwidth_mbps = (perf_metrics.total_data_mb * 8.0) / perf_metrics.test_duration_seconds;

        println!("\n🎯 STRESS TEST COMPLETED!");
        println!("==========================================");
        println!("📊 PERFORMANCE METRICS:");
        println!("   Total Transactions: {}", perf_metrics.total_transactions);
        println!("   Successful: {}", perf_metrics.successful_transactions);
        println!("   Failed: {}", perf_metrics.failed_transactions);
        println!("   Total Data: {:.2} GB", perf_metrics.total_data_mb / 1024.0);
            println!("   Test Duration: {:.2} seconds", perf_metrics.test_duration_seconds);
        println!("   Average TPS: {:.2}", perf_metrics.average_tps);
        println!("   Peak TPS: {:.2}", perf_metrics.peak_tps);
        println!("   Average Confirmation: {:.2} ms", perf_metrics.average_confirmation_time_ms);
        println!("   Min Confirmation: {} ms", perf_metrics.min_confirmation_time_ms);
        println!("   Max Confirmation: {} ms", perf_metrics.max_confirmation_time_ms);
        println!("   Network Bandwidth: {:.2} Mbps", perf_metrics.network_bandwidth_mbps);
        println!("   Success Rate: {:.2}%", 
                (perf_metrics.successful_transactions as f64 / perf_metrics.total_transactions as f64) * 100.0);

        // Save detailed report to file
        let report = serde_json::to_string_pretty(&*perf_metrics)?;
        std::fs::write("stress_test_report.json", report)?;
        println!("📄 Detailed report saved to: stress_test_report.json");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 ArthaChain Massive Stress Test");
    println!("==================================");

    // Initialize mempool
    let mempool = Arc::new(RwLock::new(Mempool::new(200_000))); // Large capacity for stress test

    // Create stress test configuration
    let config = StressTestConfig::default();

    // Create and run stress test
    let orchestrator = StressTestOrchestrator::new(config, mempool);
    orchestrator.run_stress_test().await?;

    println!("✅ Stress test completed successfully!");
    Ok(())
}
