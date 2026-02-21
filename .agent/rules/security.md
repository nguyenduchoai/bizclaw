# Security Rules - ALWAYS FOLLOW

> These rules are MANDATORY for all code generation and editing.

## 🔐 Secrets & Credentials

### NEVER Hardcode
```
❌ FORBIDDEN:
- API keys in source code
- Database passwords in config files
- JWT secrets in repositories
- OAuth client secrets exposed
- Private keys in version control
```

### ALWAYS Use
```
✅ REQUIRED:
- Environment variables for secrets
- .env files (gitignored)
- Secret managers (Vault, AWS Secrets Manager)
- Encrypted configuration
```

### Pre-commit Check
Before ANY commit, verify:
```bash
# Search for potential secrets
grep -rn "password\|secret\|api_key\|apikey\|token" --include="*.ts" --include="*.js" --include="*.json" .

# Check for .env in git
git ls-files | grep -i "\.env"
```

## 🛡️ Input Validation

### ALWAYS Sanitize
```typescript
// ✅ CORRECT: Sanitize all user input
import DOMPurify from 'dompurify';
const safeHtml = DOMPurify.sanitize(userInput);

// ✅ CORRECT: Validate and limit
const sanitizedInput = input.trim().substring(0, 4000);
```

### NEVER Trust
```typescript
// ❌ FORBIDDEN: Direct innerHTML
element.innerHTML = userInput;

// ❌ FORBIDDEN: Unsanitized SQL
const query = `SELECT * FROM users WHERE name = '${userInput}'`;

// ❌ FORBIDDEN: Eval user input
eval(userInput);
```

## 🔒 Authentication & Authorization

### Session Security
- Use secure, httpOnly cookies
- Implement CSRF protection
- Set proper SameSite attributes
- Use short-lived tokens with refresh

### Authorization Checks
```typescript
// ✅ ALWAYS verify ownership
async function getResource(userId: string, resourceId: string) {
  const resource = await db.resource.findUnique({ where: { id: resourceId } });
  if (resource.ownerId !== userId) {
    throw new ForbiddenError('Not authorized');
  }
  return resource;
}
```

## 🚫 Sensitive Data Logging

### NEVER Log
```typescript
// ❌ FORBIDDEN
console.log('User password:', password);
console.log('API Response:', JSON.stringify(response)); // May contain tokens
console.log('Request body:', req.body); // May contain PII
```

### ALWAYS Redact
```typescript
// ✅ CORRECT
console.log('User authenticated:', userId);
console.log('Request received for:', endpoint);
```

## 📋 OWASP Top 10 Checklist

Before deployment, verify protection against:
1. [ ] Injection (SQL, NoSQL, Command)
2. [ ] Broken Authentication
3. [ ] Sensitive Data Exposure
4. [ ] XML External Entities (XXE)
5. [ ] Broken Access Control
6. [ ] Security Misconfiguration
7. [ ] Cross-Site Scripting (XSS)
8. [ ] Insecure Deserialization
9. [ ] Using Components with Known Vulnerabilities
10. [ ] Insufficient Logging & Monitoring

## ⚠️ Enforcement

These rules are NON-NEGOTIABLE. Any code that violates these rules must be fixed before proceeding to other tasks.
