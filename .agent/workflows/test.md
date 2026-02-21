---
description: Chạy tests và tạo QA report
---

# /test - Testing & QA

## Workflow

### Step 1: Load QA Role
// turbo
```bash
cat .agent/roles/qa-engineer.md
```

### Step 2: Run Automated Tests
// turbo
```bash
# Try common test commands
npm test 2>/dev/null || php artisan test 2>/dev/null || pytest 2>/dev/null || go test ./... 2>/dev/null || echo "Please specify test command"
```

### Step 3: Analyze Results
- Parse test output
- Identify failing tests
- Check coverage

### Step 4: Manual Testing (if needed)
- Test edge cases not covered by automated tests
- Verify user flows
- Check error messages

### Step 5: Create QA Report
- Tạo report tại `plans/reports/qa-report-{feature}.md`

### Step 6: Output

```
🔍 QA Report

## Test Results
| Type | Total | Pass | Fail |
|------|-------|------|------|
| Unit | {n} | {n} | {n} |
| Integration | {n} | {n} | {n} |

## Coverage: {%}

## Status: ✅ PASS | ❌ FAIL

## Failing Tests (if any)
1. `{test name}` - {reason}
2. ...

## Bugs Found
| ID | Title | Severity |
|----|-------|----------|
| BUG-001 | {title} | High |

## Sign-off
- [ ] All critical tests passing
- [ ] No P0/P1 bugs
- [ ] Security tests passed
- [ ] Performance acceptable

## Recommendation
{APPROVE for release / FIX REQUIRED with list}
```

## Test Types

### Unit Tests
- Test individual functions/methods
- Mock external dependencies
- Fast, isolated

### Integration Tests
- Test component interactions
- Use real dependencies where possible
- Slower but more realistic

### E2E Tests
- Test complete user flows
- Browser/API automation
- Slower, catch integration issues

## Edge Cases to Test

- [ ] Empty inputs
- [ ] Very large inputs
- [ ] Invalid data types
- [ ] Concurrent access
- [ ] Network failures
- [ ] Timeout scenarios
- [ ] Permission denied
- [ ] Rate limiting
