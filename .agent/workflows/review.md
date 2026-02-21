---
description: Review code và đảm bảo chất lượng
---

# /review - Code Review

## Workflow

### Step 1: Load Reviewer Role
// turbo
```bash
cat .agent/roles/code-reviewer.md
```

### Step 2: Identify Files to Review
// turbo
```bash
# Xem các file thay đổi gần đây
git diff --name-only HEAD~5 2>/dev/null || find . -type f -mtime -1 | head -20
```

### Step 3: Review Checklist

#### 🔒 Security
- [ ] No hardcoded secrets/credentials
- [ ] Input validation present
- [ ] SQL injection prevention
- [ ] XSS prevention
- [ ] Proper authentication/authorization

#### ✅ Correctness
- [ ] Logic is correct
- [ ] Edge cases handled
- [ ] Error handling appropriate
- [ ] Null/undefined checks

#### 📖 Readability
- [ ] Clear naming conventions
- [ ] Appropriate comments
- [ ] Functions not too long
- [ ] Complexity manageable

#### 🏗️ Architecture
- [ ] Single responsibility
- [ ] No code duplication
- [ ] Follows existing patterns

#### ⚡ Performance
- [ ] No obvious inefficiencies
- [ ] Database queries optimized
- [ ] No memory leaks

#### 🧪 Testing
- [ ] Tests exist and meaningful
- [ ] Adequate coverage

### Step 4: Create Review Report
- Tạo report tại `plans/reports/code-review-{feature}.md`
- Categorize issues by severity

### Step 5: Output

```
👁️ Code Review Complete

## Summary
- Files reviewed: {count}
- Issues found: {count}
- Verdict: ✅ APPROVED | 🔄 CHANGES REQUESTED | ❌ REJECTED

## Issues by Severity
| Severity | Count |
|----------|-------|
| 🚫 Blocker | {n} |
| ❌ Critical | {n} |
| ⚠️ Major | {n} |
| 💡 Minor | {n} |

## Top Issues
1. **{file}:{line}** - {issue}
2. ...

## 👍 Good Practices Found
- {What was done well}

## Next Steps
{What needs to be fixed / can proceed to deploy}
```

## Issue Severity Guide

| Level | Icon | Example |
|-------|------|---------|
| Blocker | 🚫 | Security vulnerability, data corruption |
| Critical | ❌ | Broken functionality, crash |
| Major | ⚠️ | Missing validation, poor error handling |
| Minor | 💡 | Style issues, redundant code |
| Suggestion | 📝 | Nice-to-have improvements |
