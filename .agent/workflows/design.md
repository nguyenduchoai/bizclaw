---
description: Thiết kế hệ thống từ PRD
---

# /design - Thiết Kế Hệ Thống

## Precondition
- Cần có PRD tại `plans/prd-{feature}.md`
- Nếu chưa có, yêu cầu user chạy `/plan` trước

## Workflow

### Step 1: Load Architect Role
// turbo
```bash
cat .agent/roles/architect.md
```

### Step 2: Đọc PRD
// turbo
```bash
# Tìm PRD files
ls -la plans/prd-*.md 2>/dev/null || echo "No PRD found"
```

### Step 3: Analyze Codebase
// turbo
```bash
# Xem cấu trúc dự án
find . -type d -maxdepth 3 | grep -v node_modules | grep -v .git | head -30
```

### Step 4: Create Design Document
- Tạo file `plans/design-{feature-name}.md` theo template trong architect.md
- Bao gồm:
  - Architecture Overview (với ASCII diagram)
  - Components & Responsibilities
  - API Design với endpoints, request/response
  - Data Models với schemas
  - Security Considerations
  - Performance Considerations
  - File Structure
  - Implementation Notes

### Step 5: Output
- Tóm tắt design cho user
- Highlight các quyết định kỹ thuật quan trọng
- Đề xuất thứ tự implementation

## Template Output

```
🏗️ Design Created: plans/design-{feature}.md

## Architecture Summary
{Brief description of the architecture}

## Key Components
1. {Component 1}: {purpose}
2. {Component 2}: {purpose}

## API Endpoints
- `POST /api/...` - {description}
- `GET /api/...` - {description}

## Data Models
- {Entity}: {key fields}

## Implementation Order
1. {Step 1}
2. {Step 2}
3. {Step 3}

## Next Steps
- [ ] Review design
- [ ] Run `/code` to start implementation
```
