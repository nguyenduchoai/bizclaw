---
description: Triển khai code theo design document
---

# /code - Triển Khai Code

## Precondition
- Cần có Design tại `plans/design-{feature}.md`
- Nếu chưa có, yêu cầu user chạy `/design` trước

## Workflow

### Step 1: Load Engineer Role
// turbo
```bash
cat .agent/roles/engineer.md
```

### Step 2: Đọc Design Document
// turbo
```bash
# Tìm design files
ls -la plans/design-*.md 2>/dev/null || echo "No design found"
```

### Step 3: Analyze Implementation Order
- Đọc section "Implementation Notes" trong design
- Xác định thứ tự implementation:
  1. Data models / Database migrations
  2. Core business logic / Services
  3. API controllers / Routes
  4. Validation & Error handling
  5. Tests

### Step 4: Implement
- Implement từng component theo thứ tự
- Sau mỗi file:
  // turbo
  ```bash
  # Kiểm tra syntax (tùy ngôn ngữ)
  # PHP: php -l {file}
  # JS/TS: npx tsc --noEmit
  # Python: python -m py_compile {file}
  ```

### Step 5: Write Tests
- Viết unit tests cho mỗi component
- Đảm bảo coverage > 80%

### Step 6: Run Tests
// turbo
```bash
# Chạy tests (tùy project)
npm test 2>/dev/null || php artisan test 2>/dev/null || pytest 2>/dev/null || echo "No test command found"
```

### Step 7: Create Implementation Report
- Tạo report tại `plans/reports/implementation-{feature}.md`

### Step 8: Output
```
👨‍💻 Implementation Complete

## Files Created
| File | Purpose | Lines |
|------|---------|-------|
| ... | ... | ... |

## Tests
- Total: {n} tests
- Passing: {n}
- Coverage: {%}

## Next Steps
- [ ] Run `/test` for QA validation
- [ ] Run `/review` for code review
```

## Coding Checklist
- [ ] Followed naming conventions
- [ ] Error handling implemented
- [ ] Input validation added
- [ ] No hardcoded secrets
- [ ] Tests written
- [ ] No console.log/print (use proper logging)
