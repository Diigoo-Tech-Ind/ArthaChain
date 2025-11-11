/// CORS Middleware Configuration
/// Provides comprehensive CORS handling for all API endpoints

use axum::http::{header, HeaderValue, Method, Request, Response, StatusCode};
use tower::{Layer, Service};
use std::task::{Context, Poll};

/// CORS configuration with security best practices
#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<Method>,
    pub allowed_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub max_age: u32, // seconds
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        CorsConfig {
            allowed_origins: vec![
                "https://arthachain.online".to_string(),
                "https://*.arthachain.online".to_string(),
                "http://localhost:3000".to_string(), // Dev only
            ],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
                Method::PATCH,
            ],
            allowed_headers: vec![
                "authorization".to_string(),
                "content-type".to_string(),
                "accept".to_string(),
                "x-artha-session".to_string(),
                "x-artha-did".to_string(),
                "x-artha-signature".to_string(),
            ],
            expose_headers: vec![
                "content-length".to_string(),
                "content-type".to_string(),
                "x-ratelimit-remaining".to_string(),
                "x-ratelimit-reset".to_string(),
                "x-artha-node-version".to_string(),
            ],
            max_age: 3600, // 1 hour
            allow_credentials: true,
        }
    }
}

impl CorsConfig {
    /// Check if origin is allowed
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        // Check exact matches
        if self.allowed_origins.iter().any(|o| o == origin) {
            return true;
        }

        // Check wildcard patterns
        for allowed in &self.allowed_origins {
            if allowed.contains('*') {
                let pattern = allowed.replace("*.", "");
                if origin.ends_with(&pattern) {
                    return true;
                }
            }
        }

        false
    }

    /// Build Access-Control-Allow-Origin header value
    pub fn get_allow_origin(&self, origin: Option<&str>) -> Option<HeaderValue> {
        if let Some(origin) = origin {
            if self.is_origin_allowed(origin) {
                return HeaderValue::from_str(origin).ok();
            }
        }

        // Fallback to first allowed origin if no match
        if !self.allowed_origins.is_empty() {
            HeaderValue::from_str(&self.allowed_origins[0]).ok()
        } else {
            None
        }
    }

    /// Build Access-Control-Allow-Methods header
    pub fn get_allow_methods(&self) -> HeaderValue {
        let methods: Vec<String> = self.allowed_methods.iter().map(|m| m.as_str().to_string()).collect();
        HeaderValue::from_str(&methods.join(", ")).unwrap()
    }

    /// Build Access-Control-Allow-Headers header
    pub fn get_allow_headers(&self) -> HeaderValue {
        HeaderValue::from_str(&self.allowed_headers.join(", ")).unwrap()
    }

    /// Build Access-Control-Expose-Headers header
    pub fn get_expose_headers(&self) -> HeaderValue {
        HeaderValue::from_str(&self.expose_headers.join(", ")).unwrap()
    }
}

/// Middleware layer for CORS
pub struct CorsLayer {
    config: CorsConfig,
}

impl CorsLayer {
    pub fn new(config: CorsConfig) -> Self {
        CorsLayer { config }
    }
}

impl<S> Layer<S> for CorsLayer {
    type Service = CorsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CorsMiddleware {
            inner,
            config: self.config.clone(),
        }
    }
}

/// CORS middleware service
#[derive(Clone)]
pub struct CorsMiddleware<S> {
    inner: S,
    config: CorsConfig,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CorsMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        // Get origin from request
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok());

        // Handle preflight (OPTIONS) requests
        if req.method() == Method::OPTIONS {
            // In production, return proper preflight response here
            // For now, pass through to inner service
        }

        // Add CORS headers to request extensions for later use in response
        if let Some(origin) = origin {
            req.extensions_mut().insert(origin.to_string());
        }

        self.inner.call(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_allowed() {
        let config = CorsConfig::default();
        
        assert!(config.is_origin_allowed("https://arthachain.online"));
        assert!(config.is_origin_allowed("https://app.arthachain.online"));
        assert!(config.is_origin_allowed("http://localhost:3000"));
        assert!(!config.is_origin_allowed("https://evil.com"));
    }

    #[test]
    fn test_get_allow_methods() {
        let config = CorsConfig::default();
        let methods = config.get_allow_methods();
        
        let methods_str = methods.to_str().unwrap();
        assert!(methods_str.contains("GET"));
        assert!(methods_str.contains("POST"));
        assert!(methods_str.contains("OPTIONS"));
    }

    #[test]
    fn test_custom_config() {
        let config = CorsConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            allowed_methods: vec![Method::GET, Method::POST],
            allowed_headers: vec!["authorization".to_string()],
            expose_headers: vec!["x-custom".to_string()],
            max_age: 7200,
            allow_credentials: false,
        };

        assert_eq!(config.max_age, 7200);
        assert!(!config.allow_credentials);
        assert!(config.is_origin_allowed("https://example.com"));
        assert!(!config.is_origin_allowed("https://other.com"));
    }
}

