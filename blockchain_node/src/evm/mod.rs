pub mod advanced_gas_metering;
pub mod backend;
pub mod database;  // Real EVM database implementation using RocksDB
pub mod execution_engine;
pub mod executor;
pub mod opcodes;
pub mod precompiled;
pub mod precompiles;
pub mod real_executor;  // Real EVM executor using revm
pub mod rpc;
pub mod runtime;
pub mod tx_executor;  // Transaction executor integration
pub mod types;

// EVM Constants
/// Default gas price (in wei per gas unit)
pub const DEFAULT_GAS_PRICE: u64 = 20_000_000_000; // 20 Gwei

/// Default gas limit for transactions
pub const DEFAULT_GAS_LIMIT: u64 = 21_000; // Standard ETH transfer

/// Block gas limit
pub const BLOCK_GAS_LIMIT: u64 = 30_000_000; // 30M gas per block

/// Maximum code size in bytes
pub const MAX_CODE_SIZE: u64 = 24_576; // 24KB
pub const NATIVE_TO_GAS_CONVERSION_RATE: u64 = 1_000_000; // 1 native token = 1M gas units

// Re-export commonly used types
pub use advanced_gas_metering::{
    AdvancedGasConfig, AdvancedGasMeter, Eip1559GasPrice, GasEstimationResult,
};
pub use backend::{EvmAccount, EvmBackend};
pub use database::EvmDatabase;  // Real EVM database
pub use executor::EvmExecutor;
pub use real_executor::RealEvmExecutor;  // Real revm-based executor
pub use rpc::EvmRpcService;
pub use runtime::{EvmExecutionContext, EvmRuntime, StepResult};
pub use types::{EvmAddress, EvmConfig, EvmError, EvmExecutionResult, EvmLog, EvmTransaction};
