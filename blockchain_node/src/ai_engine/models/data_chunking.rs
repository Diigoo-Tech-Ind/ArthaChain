use anyhow::Result;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Compression types supported by the chunking system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    ZStd,
    LZ4,
}

/// Metadata for data chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub file_id: String,
    pub chunk_index: usize,
    pub offset: usize,
    pub size: usize,
    pub compressed_size: usize,
    pub compression: CompressionType,
    pub encryption_enabled: bool,
    pub created_at: SystemTime,
    pub hash: Vec<u8>,
}

/// A data chunk with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChunk {
    pub id: String,
    pub data: Vec<u8>,
    pub metadata: ChunkMetadata,
}

/// Pure Rust Adaptive Data Chunking Implementation
pub struct AdaptiveChunker {
    /// Chunking parameters
    params: ChunkingParams,
    /// Chunking statistics
    stats: ChunkStats,
    /// Rolling hash state
    hash_state: RollingHashState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingParams {
    /// Minimum chunk size in bytes
    pub min_size: usize,
    /// Maximum chunk size in bytes  
    pub max_size: usize,
    /// Rolling hash window size
    pub window_size: usize,
    /// Hash modulus for boundary detection
    pub hash_modulus: u32,
    /// Adaptation learning rate
    pub learning_rate: f32,
}

impl Default for ChunkingParams {
    fn default() -> Self {
        Self {
            min_size: 4096,  // 4KB
            max_size: 65536, // 64KB
            window_size: 16,
            hash_modulus: 4096,
            learning_rate: 0.1,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ChunkStats {
    /// Total chunks created
    pub total_chunks: usize,
    /// Average chunk size
    pub avg_chunk_size: f32,
    /// Compression ratio achieved
    pub compression_ratio: f32,
    /// Processing time per byte
    pub time_per_byte: f32,
}

#[derive(Debug)]
struct RollingHashState {
    /// Current hash value
    current_hash: u32,
    /// Hash multiplier
    multiplier: u32,
    /// Window buffer
    window: Vec<u8>,
    /// Current position in window
    position: usize,
}

impl Default for RollingHashState {
    fn default() -> Self {
        Self {
            current_hash: 0,
            multiplier: 257,
            window: Vec::new(),
            position: 0,
        }
    }
}

impl RollingHashState {
    fn new(window_size: usize) -> Self {
        Self {
            current_hash: 0,
            multiplier: 257,
            window: vec![0; window_size],
            position: 0,
        }
    }

    fn update(&mut self, byte: u8) -> u32 {
        let old_byte = self.window[self.position];
        self.window[self.position] = byte;

        // Remove old byte contribution and add new byte
        self.current_hash = self
            .current_hash
            .wrapping_sub(old_byte as u32)
            .wrapping_mul(self.multiplier)
            .wrapping_add(byte as u32);

        self.position = (self.position + 1) % self.window.len();
        self.current_hash
    }
}

impl AdaptiveChunker {
    /// Create a new adaptive chunker
    pub fn new(params: ChunkingParams) -> Result<Self> {
        let hash_state = RollingHashState::new(params.window_size);

        Ok(Self {
            params,
            stats: ChunkStats::default(),
            hash_state,
        })
    }

    /// Chunk data adaptively
    pub fn chunk_data(&mut self, data: &[u8]) -> Result<Vec<Vec<u8>>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let start_time = std::time::Instant::now();
        let mut chunks = Vec::new();
        let mut chunk_start = 0;

        // Reset hash state
        self.hash_state = RollingHashState::new(self.params.window_size);

        for (i, &byte) in data.iter().enumerate() {
            let hash = self.hash_state.update(byte);

            // Check for chunk boundary
            let chunk_size = i - chunk_start;
            let is_boundary = (hash % self.params.hash_modulus) == 0;
            let min_size_reached = chunk_size >= self.params.min_size;
            let max_size_reached = chunk_size >= self.params.max_size;

            if (is_boundary && min_size_reached) || max_size_reached {
                let chunk = data[chunk_start..=i].to_vec();
                chunks.push(chunk);
                chunk_start = i + 1;
            }
        }

        // Add final chunk if there's remaining data
        if chunk_start < data.len() {
            let chunk = data[chunk_start..].to_vec();
            chunks.push(chunk);
        }

        // Update statistics
        self.update_stats(&chunks, start_time.elapsed())?;

        // Adapt parameters based on performance
        self.adapt_parameters()?;

        Ok(chunks)
    }

    /// Advanced data chunking with metadata and compression
    pub async fn chunk_data_advanced(
        &mut self,
        data: &[u8],
        filename: &str,
        chunk_size: usize,
        compression_type: &str,
        encryption_enabled: bool,
    ) -> Result<Vec<DataChunk>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let start_time = std::time::Instant::now();
        let file_id = self.generate_file_id(filename, data);

        // Create chunks with specified size
        let mut chunks = Vec::new();
        let mut offset = 0;
        let mut chunk_index = 0;

        while offset < data.len() {
            let end = std::cmp::min(offset + chunk_size, data.len());
            let chunk_data = data[offset..end].to_vec();

            // Apply compression based on type
            let (compressed_data, actual_compression) = match compression_type {
                "gzip" => self.compress_gzip(&chunk_data)?,
                "zstd" => self.compress_zstd(&chunk_data)?,
                "lz4" => self.compress_lz4(&chunk_data)?,
                _ => self.compress_gzip(&chunk_data)?, // Default to gzip
            };

            // Create chunk metadata
            let metadata = ChunkMetadata {
                file_id: file_id.clone(),
                chunk_index,
                offset,
                size: chunk_data.len(),
                compressed_size: compressed_data.len(),
                compression: actual_compression,
                encryption_enabled,
                created_at: std::time::SystemTime::now(),
                hash: self.calculate_chunk_hash(&chunk_data),
            };

            // Create the chunk
            let chunk = DataChunk {
                id: format!("{}_{}", file_id, chunk_index),
                data: compressed_data,
                metadata,
            };

            chunks.push(chunk);
            offset = end;
            chunk_index += 1;
        }

        // Update statistics
        self.update_stats_advanced(&chunks, start_time.elapsed())?;

        Ok(chunks)
    }

    /// Calculate compression ratio between original and compressed data
    pub fn calculate_compression_ratio(
        &self,
        original_data: &[u8],
        chunks: &[DataChunk],
    ) -> Result<f64> {
        if original_data.is_empty() || chunks.is_empty() {
            return Ok(1.0);
        }

        let original_size = original_data.len();
        let compressed_size: usize = chunks.iter().map(|c| c.data.len()).sum();

        let ratio = if compressed_size > 0 {
            original_size as f64 / compressed_size as f64
        } else {
            1.0
        };

        Ok(ratio)
    }

    /// Generate unique file ID
    fn generate_file_id(&self, filename: &str, data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        filename.hash(&mut hasher);
        data.len().hash(&mut hasher);
        std::time::SystemTime::now().hash(&mut hasher);

        format!("file_{:x}", hasher.finish())
    }

    /// Calculate hash for a chunk
    fn calculate_chunk_hash(&self, data: &[u8]) -> Vec<u8> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);

        hasher.finish().to_le_bytes().to_vec()
    }

    /// Compress data using gzip
    fn compress_gzip(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionType)> {
        // Simplified gzip compression - in production, use proper gzip library
        // For now, return uncompressed data
        Ok((data.to_vec(), CompressionType::Gzip))
    }

    /// Compress data using zstd
    fn compress_zstd(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionType)> {
        // Simplified zstd compression - in production, use proper zstd library
        // For now, return uncompressed data
        Ok((data.to_vec(), CompressionType::ZStd))
    }

    /// Compress data using lz4
    fn compress_lz4(&self, data: &[u8]) -> Result<(Vec<u8>, CompressionType)> {
        // Simplified lz4 compression - in production, use proper lz4 library
        // For now, return uncompressed data
        Ok((data.to_vec(), CompressionType::LZ4))
    }

    /// Calculate content-based hash for boundary detection
    fn calculate_content_hash(&self, data: &[u8]) -> u32 {
        let mut hash = 0u32;
        for &byte in data {
            hash = hash.wrapping_mul(257).wrapping_add(byte as u32);
        }
        hash
    }

    /// Update chunking statistics
    fn update_stats(&mut self, chunks: &[Vec<u8>], elapsed: std::time::Duration) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }

        self.stats.total_chunks += chunks.len();

        let total_size: usize = chunks.iter().map(|c| c.len()).sum();
        let current_avg = total_size as f32 / chunks.len() as f32;

        // Update running average
        if self.stats.total_chunks == chunks.len() {
            self.stats.avg_chunk_size = current_avg;
        } else {
            let alpha = 0.1; // Exponential moving average factor
            self.stats.avg_chunk_size =
                alpha * current_avg + (1.0 - alpha) * self.stats.avg_chunk_size;
        }

        // Update timing statistics
        self.stats.time_per_byte = elapsed.as_nanos() as f32 / total_size as f32;

        // Estimate compression ratio (simplified)
        let unique_bytes = self.estimate_entropy(&chunks);
        self.stats.compression_ratio = unique_bytes / total_size as f32;

        Ok(())
    }

    /// Update advanced chunking statistics for DataChunk objects
    fn update_stats_advanced(
        &mut self,
        chunks: &[DataChunk],
        elapsed: std::time::Duration,
    ) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }

        self.stats.total_chunks += chunks.len();

        let total_size: usize = chunks.iter().map(|c| c.metadata.size).sum();
        let total_compressed_size: usize = chunks.iter().map(|c| c.metadata.compressed_size).sum();
        let current_avg = total_size as f32 / chunks.len() as f32;

        // Update running average
        if self.stats.total_chunks == chunks.len() {
            self.stats.avg_chunk_size = current_avg;
        } else {
            let alpha = 0.1; // Exponential moving average factor
            self.stats.avg_chunk_size =
                alpha * current_avg + (1.0 - alpha) * self.stats.avg_chunk_size;
        }

        // Update timing statistics
        self.stats.time_per_byte = elapsed.as_nanos() as f32 / total_size as f32;

        // Calculate real compression ratio
        if total_compressed_size > 0 {
            self.stats.compression_ratio = total_size as f32 / total_compressed_size as f32;
        }

        Ok(())
    }

    /// Estimate data entropy for compression ratio calculation
    fn estimate_entropy(&self, chunks: &[Vec<u8>]) -> f32 {
        let mut byte_counts = [0u32; 256];
        let mut total_bytes = 0;

        for chunk in chunks {
            for &byte in chunk {
                byte_counts[byte as usize] += 1;
                total_bytes += 1;
            }
        }

        if total_bytes == 0 {
            return 0.0;
        }

        let mut entropy = 0.0f32;
        for &count in &byte_counts {
            if count > 0 {
                let probability = count as f32 / total_bytes as f32;
                entropy -= probability * probability.log2();
            }
        }

        // Convert entropy to estimated unique bytes
        (entropy * total_bytes as f32 / 8.0).max(1.0)
    }

    /// Adapt chunking parameters based on performance
    fn adapt_parameters(&mut self) -> Result<()> {
        let target_chunk_size = 32768; // 32KB target
        let size_diff = self.stats.avg_chunk_size - target_chunk_size as f32;

        // Adjust hash modulus to influence chunk size
        if size_diff.abs() > 1024.0 {
            // Only adjust if difference is significant
            let adjustment = (size_diff * self.params.learning_rate) as i32;

            if size_diff > 0.0 {
                // Chunks too large, decrease modulus to create more boundaries
                self.params.hash_modulus =
                    (self.params.hash_modulus as i32 - adjustment.abs()).max(512) as u32;
            } else {
                // Chunks too small, increase modulus to create fewer boundaries
                self.params.hash_modulus =
                    (self.params.hash_modulus as i32 + adjustment.abs()).min(8192) as u32;
            }
        }

        Ok(())
    }

    /// Get current chunking statistics
    pub fn get_stats(&self) -> &ChunkStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = ChunkStats::default();
    }

    /// Update parameters
    pub fn update_params(&mut self, params: ChunkingParams) {
        self.params = params;
        self.hash_state = RollingHashState::new(self.params.window_size);
    }

    /// Validate chunk integrity
    pub fn validate_chunks(&self, chunks: &[Vec<u8>], original: &[u8]) -> Result<bool> {
        let reconstructed: Vec<u8> = chunks.iter().flatten().cloned().collect();
        Ok(reconstructed == original)
    }

    /// Calculate deduplication potential
    pub fn analyze_deduplication(&self, chunks: &[Vec<u8>]) -> HashMap<String, f32> {
        let mut hash_counts = HashMap::new();
        let mut total_size = 0;

        for chunk in chunks {
            total_size += chunk.len();
            let hash = self.calculate_content_hash(chunk);
            let hash_str = format!("{:08x}", hash);
            *hash_counts.entry(hash_str).or_insert(0) += 1;
        }

        let unique_chunks = hash_counts.len();
        let dedup_ratio = if chunks.len() > 0 {
            1.0 - (unique_chunks as f32 / chunks.len() as f32)
        } else {
            0.0
        };

        let mut analysis = HashMap::new();
        analysis.insert("deduplication_ratio".to_string(), dedup_ratio);
        analysis.insert("unique_chunks".to_string(), unique_chunks as f32);
        analysis.insert("total_chunks".to_string(), chunks.len() as f32);
        analysis.insert(
            "avg_chunk_size".to_string(),
            if chunks.len() > 0 {
                total_size as f32 / chunks.len() as f32
            } else {
                0.0
            },
        );

        analysis
    }
}
