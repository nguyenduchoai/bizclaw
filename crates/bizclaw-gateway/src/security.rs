use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::Duration;
use tokio::sync::RateLimiter;
use tower_http::cors::{Any, CorsLayer};
use std::net::IpAddr;

#[derive(Clone)]
pub struct SecurityConfig {
    pub rate_limit_per_minute: u32,
    pub max_request_size_mb: usize,
    pub allowed_origins: Vec<String>,
    pub enable_hsts: bool,
    pub enable_csp: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit_per_minute: 60,
            max_request_size_mb: 10,
            allowed_origins: vec!["*".to_string()],
            enable_hsts: true,
            enable_csp: true,
        }
    }
}

pub fn cors_layer(config: &SecurityConfig) -> CorsLayer {
    let allowed_origins: Vec<String> = config.allowed_origins.clone();
    
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any)
        .max_age(Duration::from_secs(86400))
}

pub async fn rate_limit_middleware(
    ip: std::net::IpAddr,
    limiter: &RateLimiter,
    next: Next,
    request: Request<Body>,
) -> Response {
    let key = ip.to_string();
    
    match limiter.try_acquire(&key) {
        Ok(_permit) => next.run(request).await,
        Err(_) => Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body(Body::from("Rate limit exceeded"))
            .unwrap(),
    }
}

pub fn security_headers() -> impl Fn(Request<Body>, Next) -> Response + Clone {
    move |_request: Request<Body>, next: Next| {
        let mut response = tower_util::ServiceExt::oneshot(&next, _request).unwrap();
        
        let headers = response.headers_mut();
        
        headers.insert("Strict-Transport-Security", 
                       "max-age=31536000; includeSubDomains; preload".parse().unwrap());
        
        headers.insert("X-Frame-Options", "DENY".parse().unwrap());
        
        headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
        
        headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
        
        headers.insert("Referrer-Policy", "strict-origin-when-cross-origin".parse().unwrap());
        
        headers.insert("Permissions-Policy", 
                       "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()".parse().unwrap());
        
        headers.insert("Content-Security-Policy", 
                       "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self' https://*.openai.com https://*.anthropic.com https://*.googleapis.com; frame-ancestors 'none';".parse().unwrap());
        
        response
    }
}

pub async fn validate_request_size(
    request: Request<Body>,
    max_size_mb: usize,
    next: Next,
) -> Response {
    let (parts, body) = request.into_parts();
    
    let content_length = parts.headers.get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    
    let max_size_bytes = max_size_mb * 1024 * 1024;
    
    if content_length > max_size_bytes {
        return Response::builder()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .body(Body::from(format!("Request body too large. Maximum size is {} MB", max_size_mb)))
            .unwrap();
    }
    
    let request = Request::from_parts(parts, body);
    next.run(request).await
}

pub async fn log_request_info(
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    
    tracing::info!(
        method = %method,
        uri = %uri,
        client_ip = %client_ip,
        "Incoming request"
    );
    
    let response = next.run(request).await;
    
    tracing::info!(
        method = %method,
        uri = %uri,
        status = %response.status(),
        "Request completed"
    );
    
    response
}

pub struct RequestValidator {
    max_body_size: usize,
    allowed_content_types: Vec<String>,
    require_auth: bool,
}

impl RequestValidator {
    pub fn new(max_body_size: usize) -> Self {
        Self {
            max_body_size,
            allowed_content_types: vec![
                "application/json".to_string(),
                "application/octet-stream".to_string(),
            ],
            require_auth: true,
        }
    }
    
    pub fn validate(&self, request: &Request<Body>) -> Result<(), String> {
        if self.require_auth {
            let auth_header = request.headers()
                .get("authorization");
            
            if auth_header.is_none() {
                return Err("Missing authorization header".to_string());
            }
        }
        
        let content_type = request.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        
        if !self.allowed_content_types.iter().any(|t| content_type.starts_with(t)) {
            return Err(format!("Invalid content type: {}", content_type));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert_eq!(config.rate_limit_per_minute, 60);
        assert_eq!(config.max_request_size_mb, 10);
    }

    #[test]
    fn test_request_validator() {
        let validator = RequestValidator::new(10 * 1024 * 1024);
        assert_eq!(validator.max_body_size, 10 * 1024 * 1024);
    }
}
