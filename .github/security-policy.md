# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.1.x   | :white_check_mark: |
| 1.0.x   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability within BizClaw, please create a private security advisory through GitHub. Please do **NOT** create a public issue for security vulnerabilities.

### What to Include

When reporting a vulnerability, please include:

1. **Description**: A clear description of the vulnerability
2. **Steps to Reproduce**: Detailed steps to reproduce the issue
3. **Impact**: What an attacker could do with this vulnerability
4. **Affected Components**: Which parts of the codebase are affected
5. **Suggested Fix**: If you have one, how you suggest fixing it

### Response Timeline

- **Initial Response**: Within 48 hours
- **Assessment**: Within 7 days
- **Fix Development**: Depending on severity
- **Security Advisory**: Published after fix is available

## Security Best Practices

### For Users

1. **Keep Updated**: Always use the latest version of BizClaw
2. **Environment Variables**: Never commit `.env` files with real credentials
3. **API Keys**: Rotate API keys regularly
4. **Network Security**: Use TLS in production
5. **Access Control**: Implement least-privilege principles

### For Developers

1. **Input Validation**: Validate all user input
2. **Output Encoding**: Encode output appropriately
3. **Authentication**: Implement proper authentication
4. **Authorization**: Check permissions before actions
5. **Logging**: Log security-relevant events
6. **Error Handling**: Don't expose sensitive information in errors

## Security Features

### Built-in Security

- [ ] Encryption at rest for sensitive data
- [ ] TLS/SSL support
- [ ] JWT token authentication
- [ ] Rate limiting
- [ ] Input sanitization
- [ ] SQL injection prevention (via parameterized queries)
- [ ] XSS protection
- [ ] CSRF protection
- [ ] Security headers (HSTS, CSP, etc.)

### Third-party Security Tools

- [ ] Trivy for container scanning
- [ ] CodeQL for static analysis
- [ ] cargo-audit for dependency vulnerabilities
- [ ] cargo-deny for dependency policy
- [ ] GitLeaks for secret scanning

## Compliance

BizClaw is designed to help SMEs comply with:

- [ ] Vietnamese PDPD (Luật An Ninh Mạng)
- [ ] GDPR (for EU customers)
- [ ] CCPA (for California customers)
- [ ] SOC 2 Type II (future)
