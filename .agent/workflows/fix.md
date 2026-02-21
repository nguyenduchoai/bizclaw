---
description: Fix bugs với systematic debugging
---

# /fix - Debug & Fix Issues

## Workflow

### Step 1: Understand the Issue
- Đọc error message / bug report
- Xác định severity (Critical/High/Medium/Low)

### Step 2: Route to Specialized Fix

**Decision Tree:**

| Issue Type | Keywords | Action |
|------------|----------|--------|
| Type errors | typescript, tsc, type error | `/fix:types` |
| UI/UX | ui, ux, design, layout, css | `/fix:ui` |
| CI/CD | github actions, pipeline, workflow | `/fix:ci` |
| Test failures | test, jest, vitest, failing | `/fix:test` |
| Log errors | logs, stack trace, error logs | `/fix:logs` |
| Complex | architecture, refactor, system-wide | `/fix:hard` |
| Simple | small bug, single file | `/fix:fast` |

### Step 3: Systematic Debugging

**4-Phase Debugging Framework:**

#### Phase 1: Root Cause Investigation
```
- [ ] Read full error message
- [ ] Reproduce the issue
- [ ] Check recent changes (git log)
- [ ] Gather evidence
```

#### Phase 2: Pattern Analysis
```
- [ ] Find working examples
- [ ] Compare with broken code
- [ ] Identify differences
```

#### Phase 3: Hypothesis & Testing
```
- [ ] Form theory about cause
- [ ] Test minimally
- [ ] Verify hypothesis
```

#### Phase 4: Implementation
```
- [ ] Create regression test
- [ ] Apply fix at root cause
- [ ] Verify fix works
```

### Step 4: Run Tests
// turbo
```bash
npm test 2>/dev/null || php artisan test 2>/dev/null || pytest 2>/dev/null
```

### Step 5: Output

```
🔧 Bug Fixed

## Issue
{Description of the issue}

## Root Cause
{What caused the issue}

## Fix Applied
{What was changed and why}

## Files Modified
| File | Changes |
|------|---------|
| {path} | {what changed} |

## Tests
- [ ] Regression test added
- [ ] All tests passing

## Verification
{Evidence that the fix works}
```

---

## /fix:fast - Quick Fix

For simple, single-file bugs:
1. Identify the issue
2. Apply fix
3. Test
4. Done

---

## /fix:hard - Complex Fix

For multi-component issues:
1. Create investigation plan
2. Analyze each component
3. Document findings
4. Propose solution
5. Get approval
6. Implement
7. Test thoroughly
8. Review

---

## Common Fix Patterns

### NullPointerException / undefined
```
Problem: Accessing property of null/undefined
Fix: Add null check or optional chaining
```

### Type Mismatch
```
Problem: Wrong type passed to function
Fix: Validate input types, add type guards
```

### Race Condition
```
Problem: Async operations in wrong order
Fix: Proper async/await, locks, or queues
```

### Memory Leak
```
Problem: Objects not released
Fix: Clean up event listeners, dispose resources
```

## Red Flags - Stop & Investigate

If thinking:
- "Quick fix for now, investigate later"
- "Just try changing X"
- "It's probably X"
- "Should work now"

→ **Return to systematic process**
