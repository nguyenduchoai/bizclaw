#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_audit_trail_creation() {
        let audit = AuditEntry::new(
            AuditAction::UserLogin,
            AuditResource::User,
            "user_123".to_string(),
        );
        
        assert_eq!(audit.action, AuditAction::UserLogin);
        assert_eq!(audit.resource, AuditResource::User);
    }

    #[test]
    fn test_input_sanitization() {
        let malicious_input = "<script>alert('xss')</script>";
        let sanitized = sanitize_input(malicious_input);
        
        assert!(!sanitized.contains("<script>"));
        assert!(sanitized.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(10, std::time::Duration::from_secs(60));
        
        for i in 0..10 {
            assert!(limiter.try_acquire(&format!("key_{}", i)).is_ok());
        }
        
        assert!(limiter.try_acquire("key_0").is_err());
    }

    #[test]
    fn test_password_hashing() {
        let password = "SecureP@ssw0rd!";
        let hash = hash_password(password).unwrap();
        
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn test_encryption_roundtrip() {
        let key = generate_encryption_key().unwrap();
        let plaintext = "Sensitive business data";
        
        let encrypted = encrypt_data(plaintext.as_bytes(), &key).unwrap();
        let decrypted = decrypt_data(&encrypted, &key).unwrap();
        
        assert_eq!(String::from_utf8(decrypted).unwrap(), plaintext);
    }

    #[test]
    fn test_jwt_validation() {
        let secret = "test_secret_key_1234567890";
        let claims = JWTClaims {
            sub: "user_123".to_string(),
            exp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600,
            iat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            role: "admin".to_string(),
        };
        
        let token = create_jwt(&claims, secret).unwrap();
        let validated = validate_jwt(&token, secret).unwrap();
        
        assert_eq!(validated.sub, "user_123");
        assert_eq!(validated.role, "admin");
    }

    #[test]
    fn test_sql_injection_prevention() {
        let malicious_input = "'; DROP TABLE users; --";
        let query = "SELECT * FROM users WHERE username = $1";
        
        let (safe_query, safe_params) = prepare_safe_query(query, malicious_input);
        
        assert!(!safe_query.contains("DROP TABLE"));
        assert!(safe_params.len() == 1);
    }

    #[test]
    fn test_cors_validation() {
        let allowed_origins = vec![
            "https://bizclaw.example.com".to_string(),
            "https://app.bizclaw.com".to_string(),
        ];
        
        assert!(validate_origin("https://bizclaw.example.com", &allowed_origins).unwrap());
        assert!(!validate_origin("https://evil.com", &allowed_origins).unwrap());
    }

    #[test]
    fn test_header_security() {
        let headers = SecurityHeaders::default();
        
        assert!(headers.strict_transport_security.contains("max-age"));
        assert_eq!(headers.x_frame_options, "DENY");
        assert_eq!(headers.x_content_type_options, "nosniff");
    }
}
