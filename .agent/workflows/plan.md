---
description: Lập kế hoạch và tạo PRD cho feature mới
---

# /plan - Lập Kế Hoạch Feature

## Workflow

### Step 1: Phân Tích Yêu Cầu
// turbo
```bash
# Đọc file product-manager.md để hiểu role
cat .agent/roles/product-manager.md
```

### Step 2: Clarify Requirements
- Sử dụng AskUserQuestion tool để hỏi thêm chi tiết nếu cần
- Hỏi từng câu một, đợi user trả lời

### Step 3: Research Context
// turbo
```bash
# Tìm hiểu codebase hiện tại
find . -type f -name "*.md" | head -20
```

### Step 4: Tạo PRD
- Tạo file `plans/prd-{feature-name}.md` theo template trong product-manager.md
- Đảm bảo có đầy đủ:
  - Problem Statement
  - Goals & Non-Goals
  - User Stories với Acceptance Criteria
  - Functional & Non-Functional Requirements
  - Dependencies & Risks
  - Success Metrics
  - Timeline

### Step 5: Output
- Tóm tắt PRD cho user
- Liệt kê câu hỏi chưa được giải đáp (nếu có)
- Hỏi user muốn tiếp tục với `/design` hay cần chỉnh sửa PRD

## Template Output

```
📋 PRD Created: plans/prd-{feature}.md

## Summary
{2-3 sentences about the feature}

## Key Requirements
1. {Requirement 1}
2. {Requirement 2}

## Timeline
- Phase 1: {milestone}

## Next Steps
- [ ] Review PRD
- [ ] Run `/design` to create system design

❓ Questions for clarification:
- {Question if any}
```
