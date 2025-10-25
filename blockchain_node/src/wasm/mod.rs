//! WebAssembly (WASM) execution engine for ArthaChain
//! 
//! This module provides a complete WASM runtime for executing smart contracts
//! written in Rust, AssemblyScript, and other WASM-compatible languages.

pub mod engine;
pub mod runtime;
pub mod gas;
pub mod storage;
pub mod host_functions;

pub use engine::WasmEngine;
pub use runtime::WasmRuntime;
pub use gas::GasMeter;
pub use storage::WasmStorage;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
#[cfg(feature = "wasm-runtime")]
use wasmtime::*;

/// Configuration for WASM execution
#[derive(Debug, Clone)]
pub struct WasmConfig {
    /// Maximum memory pages (64KB each)
    pub max_memory_pages: u32,
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Maximum stack size
    pub max_stack_size: usize,
    /// Gas limit for WASM execution
    pub gas_limit: u64,
    /// Enable optimizations
    pub enable_optimizations: bool,
    /// Enable debugging
    pub enable_debugging: bool,
    /// Enable profiling
    pub enable_profiling: bool,
    /// Compiler optimization level
    pub optimization_level: u8,
    /// Enable SIMD support
    pub enable_simd: bool,
    /// Enable threads support
    pub enable_threads: bool,
    /// Enable bulk memory operations
    pub enable_bulk_memory: bool,
    /// Enable reference types
    pub enable_reference_types: bool,
    /// Enable multi-value returns
    pub enable_multi_value: bool,
    /// Enable tail calls
    pub enable_tail_calls: bool,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            max_memory_pages: 1024, // 64MB
            max_execution_time_ms: 5000, // 5 seconds
            max_stack_size: 1024 * 1024, // 1MB
            gas_limit: 100_000_000,
            enable_optimizations: true,
            enable_debugging: false,
            enable_profiling: false,
            optimization_level: 2,
            enable_simd: true,
            enable_threads: true,
            enable_bulk_memory: true,
            enable_reference_types: true,
            enable_multi_value: true,
            enable_tail_calls: true,
        }
    }
}

/// WASM execution result
#[derive(Debug, Clone)]
pub struct WasmExecutionResult {
    /// Execution success status
    pub success: bool,
    /// Return data from the contract
    pub return_data: Vec<u8>,
    /// Gas consumed during execution
    pub gas_consumed: u64,
    /// Memory pages allocated
    pub memory_pages_allocated: u32,
    /// Execution time in microseconds
    pub execution_time_us: u64,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Debug information
    pub debug_info: Option<WasmDebugInfo>,
    /// Profiling data
    pub profiling_data: Option<WasmProfilingData>,
}

/// Debug information for WASM execution
#[derive(Debug, Clone)]
pub struct WasmDebugInfo {
    /// Stack trace
    pub stack_trace: Vec<String>,
    /// Memory usage
    pub memory_usage: WasmMemoryUsage,
    /// Function calls
    pub function_calls: Vec<WasmFunctionCall>,
    /// Breakpoints hit
    pub breakpoints_hit: Vec<u32>,
}

/// Memory usage information
#[derive(Debug, Clone)]
pub struct WasmMemoryUsage {
    /// Current memory usage in bytes
    pub current_bytes: usize,
    /// Peak memory usage in bytes
    pub peak_bytes: usize,
    /// Memory pages used
    pub pages_used: u32,
    /// Memory fragmentation percentage
    pub fragmentation_percent: f64,
}

/// Function call information
#[derive(Debug, Clone)]
pub struct WasmFunctionCall {
    /// Function name
    pub name: String,
    /// Entry point
    pub entry_point: u32,
    /// Execution time in microseconds
    pub execution_time_us: u64,
    /// Gas consumed
    pub gas_consumed: u64,
    /// Parameters
    pub parameters: Vec<WasmValue>,
    /// Return values
    pub return_values: Vec<WasmValue>,
}

/// WASM value types
#[derive(Debug, Clone)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128([u8; 16]),
    FuncRef(Option<u32>),
    ExternRef(Option<u32>),
}

/// Profiling data for WASM execution
#[derive(Debug, Clone)]
pub struct WasmProfilingData {
    /// Function execution times
    pub function_times: std::collections::HashMap<String, u64>,
    /// Memory allocation events
    pub memory_allocations: Vec<WasmMemoryAllocation>,
    /// Cache hit/miss statistics
    pub cache_stats: WasmCacheStats,
    /// Performance counters
    pub performance_counters: std::collections::HashMap<String, u64>,
}

/// Memory allocation event
#[derive(Debug, Clone)]
pub struct WasmMemoryAllocation {
    /// Allocation size in bytes
    pub size: usize,
    /// Allocation timestamp
    pub timestamp: u64,
    /// Allocation stack trace
    pub stack_trace: Vec<String>,
    /// Deallocation timestamp (if freed)
    pub deallocation_timestamp: Option<u64>,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct WasmCacheStats {
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Cache hit ratio
    pub hit_ratio: f64,
    /// Cache size in bytes
    pub cache_size: usize,
    /// Evictions
    pub evictions: u64,
}

/// WASM contract interface
pub trait WasmContract {
    /// Initialize the contract
    fn init(&mut self, config: &WasmConfig) -> Result<()>;
    
    /// Execute a function call
    fn execute_function(
        &mut self,
        function_name: &str,
        parameters: Vec<WasmValue>,
        gas_limit: u64,
    ) -> Result<WasmExecutionResult>;
    
    /// Get contract metadata
    fn get_metadata(&self) -> WasmContractMetadata;
    
    /// Validate contract code
    fn validate(&self, code: &[u8]) -> Result<()>;
    
    /// Compile contract code
    fn compile(&self, code: &[u8], config: &WasmConfig) -> Result<Vec<u8>>;
}

/// Contract metadata
#[derive(Debug, Clone)]
pub struct WasmContractMetadata {
    /// Contract name
    pub name: String,
    /// Contract version
    pub version: String,
    /// Contract author
    pub author: String,
    /// Contract description
    pub description: String,
    /// Exported functions
    pub exported_functions: Vec<WasmFunctionExport>,
    /// Imported functions
    pub imported_functions: Vec<WasmFunctionImport>,
    /// Memory requirements
    pub memory_requirements: WasmMemoryRequirements,
    /// Gas costs per function
    pub gas_costs: std::collections::HashMap<String, u64>,
}

/// Function export information
#[derive(Debug, Clone)]
pub struct WasmFunctionExport {
    /// Function name
    pub name: String,
    /// Function signature
    pub signature: WasmFunctionSignature,
    /// Function type
    pub function_type: WasmFunctionType,
    /// Gas cost
    pub gas_cost: u64,
}

/// Function import information
#[derive(Debug, Clone)]
pub struct WasmFunctionImport {
    /// Module name
    pub module: String,
    /// Function name
    pub name: String,
    /// Function signature
    pub signature: WasmFunctionSignature,
    /// Function type
    pub function_type: WasmFunctionType,
}

/// Function signature
#[derive(Debug, Clone)]
pub struct WasmFunctionSignature {
    /// Parameter types
    pub parameters: Vec<WasmType>,
    /// Return types
    pub returns: Vec<WasmType>,
}

/// WASM type definitions
#[derive(Debug, Clone, PartialEq)]
pub enum WasmType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef,
}

/// Function type
#[derive(Debug, Clone)]
pub enum WasmFunctionType {
    /// Regular function
    Function,
    /// Constructor
    Constructor,
    /// Destructor
    Destructor,
    /// Getter
    Getter,
    /// Setter
    Setter,
    /// Event handler
    EventHandler,
    /// Fallback function
    Fallback,
}

/// Memory requirements
#[derive(Debug, Clone)]
pub struct WasmMemoryRequirements {
    /// Minimum memory pages
    pub min_pages: u32,
    /// Maximum memory pages
    pub max_pages: u32,
    /// Initial memory pages
    pub initial_pages: u32,
    /// Memory alignment
    pub alignment: u32,
}

impl Default for WasmMemoryRequirements {
    fn default() -> Self {
        Self {
            min_pages: 1,
            max_pages: 1024,
            initial_pages: 1,
            alignment: 16,
        }
    }
}