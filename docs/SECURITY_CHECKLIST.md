# Security Checklist

## Development Phase

### Code Review
- [ ] No hardcoded secrets or credentials
- [ ] Input validation on all user inputs
- [ ] Output encoding for all outputs
- [ ] Proper error handling (no stack traces in production)
- [ ] Secure random number generation
- [ ] No use of `unsafe` without justification
- [ ] Proper authentication checks
- [ ] Authorization checks before sensitive operations

### Dependencies
- [ ] All dependencies are from trusted sources
- [ ] No dependencies with known vulnerabilities
- [ ] Dependencies are regularly updated
- [ ] No unnecessary dependencies
- [ ] License compliance verified

### Testing
- [ ] Unit tests for security-critical functions
- [ ] Integration tests for authentication flows
- [ ] Penetration testing completed
- [ ] Fuzzing tests for parsers
- [ ] Load testing under attack scenarios

## Build Phase

### CI/CD Security
- [ ] Secrets stored in environment variables or secret management
- [ ] Build artifacts verified (checksums, signatures)
- [ ] No credentials in build logs
- [ ] Build reproducible
- [ ] Container images scanned for vulnerabilities

### Static Analysis
- [ ] CodeQL analysis passes
- [ ] Cargo audit finds no critical vulnerabilities
- [ ] Cargo deny policies enforced
- [ ] No new warnings introduced

## Deployment Phase

### Configuration
- [ ] Production credentials rotated
- [ ] Debug mode disabled
- [ ] Error details not exposed to users
- [ ] Logging configured (but not exposing secrets)
- [ ] TLS certificates valid and up-to-date
- [ ] Security headers configured
- [ ] CORS configured properly
- [ ] Rate limiting enabled

### Infrastructure
- [ ] Firewall rules configured
- [ ] Database not exposed publicly
- [ ] API accessible only via HTTPS
- [ ] Backup encryption enabled
- [ ] Monitoring and alerting configured
- [ ] Incident response plan documented

### Access Control
- [ ] Default passwords changed
- [ ] Root/admin access requires MFA
- [ ] Least-privilege principle applied
- [ ] Regular access reviews scheduled
- [ ] Audit logging enabled

## Operations Phase

### Monitoring
- [ ] Security events logged and monitored
- [ ] Anomaly detection active
- [ ] Log retention policy configured
- [ ] Alerts for suspicious activity

### Maintenance
- [ ] Regular security updates applied
- [ ] Dependency updates reviewed
- [ ] Security patches prioritized
- [ ] Vulnerability scans performed regularly

### Incident Response
- [ ] Incident response plan documented
- [ ] Contact information current
- [ ] Forensic capabilities in place
- [ ] Communication templates prepared

## Compliance

### Data Protection
- [ ] PII handling documented
- [ ] Data retention policy defined
- [ ] Encryption at rest enabled
- [ ] Encryption in transit enforced
- [ ] Backup encryption enabled

### Regulations
- [ ] GDPR compliance (if applicable)
- [ ] Vietnamese PDPD compliance
- [ ] Industry-specific regulations met
- [ ] Data processing agreements in place

### Audits
- [ ] Regular security audits scheduled
- [ ] Third-party penetration testing
- [ ] Compliance audits completed
- [ ] Audit reports retained

## Vietnam-Specific Considerations

### Local Regulations
- [ ] PDPD compliance for data localization
- [ ] API compliance for cloud services
- [ ] Cybersecurity Law requirements met

### Data Residency
- [ ] Customer data stored in Vietnam (if required)
- [ ] Cross-border data transfer policies
- [ ] Data processing location documented

### Business Operations
- [ ] Local support channels available
- [ ] Payment compliance (VNPay, MoMo, etc.)
- [ ] Tax compliance (e-invoices)
