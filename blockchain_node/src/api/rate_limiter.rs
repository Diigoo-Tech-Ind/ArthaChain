/// Rate Limiting Middleware for Artha API endpoints
/// Implements per-DID, per-IP, and global rate limits

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub per_ip_per_second: u32,
    pub per_ip_per_minute: u32,
    pub per_ip_per_hour: u32,
    pub per_did_per_second: u32,
    pub per_did_per_hour: u32,
    pub global_per_second: u32,
    pub burst_allowance: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            per_ip_per_second: 10,
            per_ip_per_minute: 300,
            per_ip_per_hour: 3000,
            per_did_per_second: 5,
            per_did_per_hour: 1000,
            global_per_second: 1000,
            burst_allowance: 20,
        }
    }
}

struct RateLimitEntry {
    count: u32,
    window_start: Instant,
    last_request: Instant,
}

impl RateLimitEntry {
    fn new() -> Self {
        let now = Instant::now();
        RateLimitEntry {
            count: 0,
            window_start: now,
            last_request: now,
        }
    }

    fn reset(&mut self) {
        self.count = 0;
        self.window_start = Instant::now();
    }

    fn increment(&mut self) {
        self.count += 1;
        self.last_request = Instant::now();
    }
}

pub struct RateLimiter {
    config: RateLimitConfig,
    ip_limits: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
    did_limits: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
    global_counter: Arc<RwLock<RateLimitEntry>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        RateLimiter {
            config,
            ip_limits: Arc::new(RwLock::new(HashMap::new())),
            did_limits: Arc::new(RwLock::new(HashMap::new())),
            global_counter: Arc::new(RwLock::new(RateLimitEntry::new())),
        }
    }

    /// Check if a request should be allowed
    pub fn check_rate_limit(&self, ip: &str, did: Option<&str>) -> Result<(), RateLimitError> {
        // Check global limit
        self.check_global()?;

        // Check IP-based limits
        self.check_ip_limit(ip)?;

        // Check DID-based limits if DID provided
        if let Some(did) = did {
            self.check_did_limit(did)?;
        }

        // Increment counters
        self.increment_counters(ip, did);

        Ok(())
    }

    fn check_global(&self) -> Result<(), RateLimitError> {
        let mut global = self.global_counter.write().unwrap();
        let now = Instant::now();

        // Reset if window passed (1 second)
        if now.duration_since(global.window_start) > Duration::from_secs(1) {
            global.reset();
        }

        if global.count >= self.config.global_per_second {
            return Err(RateLimitError::GlobalLimit);
        }

        Ok(())
    }

    fn check_ip_limit(&self, ip: &str) -> Result<(), RateLimitError> {
        let mut ip_limits = self.ip_limits.write().unwrap();
        let entry = ip_limits.entry(ip.to_string()).or_insert_with(RateLimitEntry::new);
        
        let now = Instant::now();
        let elapsed = now.duration_since(entry.window_start);

        // Per-second limit
        if elapsed < Duration::from_secs(1) {
            if entry.count >= self.config.per_ip_per_second + self.config.burst_allowance {
                return Err(RateLimitError::IPPerSecond);
            }
        }

        // Per-minute limit
        if elapsed < Duration::from_secs(60) {
            if entry.count >= self.config.per_ip_per_minute {
                return Err(RateLimitError::IPPerMinute);
            }
        }

        // Per-hour limit
        if elapsed < Duration::from_secs(3600) {
            if entry.count >= self.config.per_ip_per_hour {
                return Err(RateLimitError::IPPerHour);
            }
        }

        // Reset window if hour passed
        if elapsed > Duration::from_secs(3600) {
            entry.reset();
        }

        Ok(())
    }

    fn check_did_limit(&self, did: &str) -> Result<(), RateLimitError> {
        let mut did_limits = self.did_limits.write().unwrap();
        let entry = did_limits.entry(did.to_string()).or_insert_with(RateLimitEntry::new);
        
        let now = Instant::now();
        let elapsed = now.duration_since(entry.window_start);

        // Per-second limit
        if elapsed < Duration::from_secs(1) {
            if entry.count >= self.config.per_did_per_second {
                return Err(RateLimitError::DIDPerSecond);
            }
        }

        // Per-hour limit
        if elapsed < Duration::from_secs(3600) {
            if entry.count >= self.config.per_did_per_hour {
                return Err(RateLimitError::DIDPerHour);
            }
        }

        // Reset window if hour passed
        if elapsed > Duration::from_secs(3600) {
            entry.reset();
        }

        Ok(())
    }

    fn increment_counters(&self, ip: &str, did: Option<&str>) {
        // Increment global
        let mut global = self.global_counter.write().unwrap();
        global.increment();

        // Increment IP
        let mut ip_limits = self.ip_limits.write().unwrap();
        if let Some(entry) = ip_limits.get_mut(ip) {
            entry.increment();
        }

        // Increment DID
        if let Some(did) = did {
            let mut did_limits = self.did_limits.write().unwrap();
            if let Some(entry) = did_limits.get_mut(did) {
                entry.increment();
            }
        }
    }

    /// Cleanup expired entries (call periodically)
    pub fn cleanup_expired(&self) {
        let now = Instant::now();

        // Cleanup IP limits
        let mut ip_limits = self.ip_limits.write().unwrap();
        ip_limits.retain(|_, entry| {
            now.duration_since(entry.last_request) < Duration::from_secs(3600)
        });

        // Cleanup DID limits
        let mut did_limits = self.did_limits.write().unwrap();
        did_limits.retain(|_, entry| {
            now.duration_since(entry.last_request) < Duration::from_secs(3600)
        });
    }

    /// Get current rate limit status for an IP
    pub fn get_ip_status(&self, ip: &str) -> Option<RateLimitStatus> {
        let ip_limits = self.ip_limits.read().unwrap();
        ip_limits.get(ip).map(|entry| {
            let elapsed = Instant::now().duration_since(entry.window_start);
            RateLimitStatus {
                count: entry.count,
                window_elapsed: elapsed,
                limit_per_second: self.config.per_ip_per_second,
                limit_per_hour: self.config.per_ip_per_hour,
            }
        })
    }

    /// Get current rate limit status for a DID
    pub fn get_did_status(&self, did: &str) -> Option<RateLimitStatus> {
        let did_limits = self.did_limits.read().unwrap();
        did_limits.get(did).map(|entry| {
            let elapsed = Instant::now().duration_since(entry.window_start);
            RateLimitStatus {
                count: entry.count,
                window_elapsed: elapsed,
                limit_per_second: self.config.per_did_per_second,
                limit_per_hour: self.config.per_did_per_hour,
            }
        })
    }
}

#[derive(Debug)]
pub enum RateLimitError {
    GlobalLimit,
    IPPerSecond,
    IPPerMinute,
    IPPerHour,
    DIDPerSecond,
    DIDPerHour,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::GlobalLimit => write!(f, "Global rate limit exceeded"),
            RateLimitError::IPPerSecond => write!(f, "IP rate limit exceeded (per second)"),
            RateLimitError::IPPerMinute => write!(f, "IP rate limit exceeded (per minute)"),
            RateLimitError::IPPerHour => write!(f, "IP rate limit exceeded (per hour)"),
            RateLimitError::DIDPerSecond => write!(f, "DID rate limit exceeded (per second)"),
            RateLimitError::DIDPerHour => write!(f, "DID rate limit exceeded (per hour)"),
        }
    }
}

impl std::error::Error for RateLimitError {}

#[derive(Debug)]
pub struct RateLimitStatus {
    pub count: u32,
    pub window_elapsed: Duration,
    pub limit_per_second: u32,
    pub limit_per_hour: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_ip_rate_limit() {
        let config = RateLimitConfig {
            per_ip_per_second: 5,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        // Should allow 5 requests
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("192.168.1.1", None).is_ok());
        }

        // 6th request should fail
        assert!(limiter.check_rate_limit("192.168.1.1", None).is_err());

        // Wait 1 second and retry
        thread::sleep(Duration::from_secs(1));
        assert!(limiter.check_rate_limit("192.168.1.1", None).is_ok());
    }

    #[test]
    fn test_did_rate_limit() {
        let config = RateLimitConfig {
            per_did_per_second: 3,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        let did = "did:artha:test123";

        // Should allow 3 requests
        for _ in 0..3 {
            assert!(limiter.check_rate_limit("192.168.1.1", Some(did)).is_ok());
        }

        // 4th request should fail
        assert!(limiter.check_rate_limit("192.168.1.1", Some(did)).is_err());
    }

    #[test]
    fn test_cleanup_expired() {
        let limiter = RateLimiter::new(RateLimitConfig::default());

        limiter.check_rate_limit("192.168.1.1", None).ok();
        limiter.check_rate_limit("192.168.1.2", None).ok();

        assert_eq!(limiter.ip_limits.read().unwrap().len(), 2);

        limiter.cleanup_expired();

        // Entries are recent, should not be cleaned up yet
        assert_eq!(limiter.ip_limits.read().unwrap().len(), 2);
    }
}

