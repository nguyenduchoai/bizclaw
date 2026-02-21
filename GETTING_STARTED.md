# 🏢 Bizino AI DEV - Getting Started

## 📦 Bước 1: Giải Nén

Giải nén file `ai-software-company.zip` vào thư mục dự án của bạn.

---

## 🚀 Bước 2: Chạy Setup

### macOS / Linux
```bash
cd /path/to/your/project
chmod +x install.sh
./install.sh
```

### Windows (Git Bash hoặc WSL)
```bash
cd /path/to/your/project
bash install.sh
```

---

## 📋 Setup Script Sẽ Tự Động:

1. **Phát hiện loại dự án** (NodeJS, Laravel, Python, Go, etc.)
2. **Setup phù hợp**:
   - Dự án có sẵn → Merge không ghi đè
   - Dự án mới → Tạo cấu trúc đầy đủ
3. **Tạo cấu hình** `.agent/project.json`
4. **Hiển thị hướng dẫn** sử dụng

---

## 🎭 Hai Trường Hợp

### Trường hợp 1: DỰ ÁN CÓ SẴN

```
📁 your-existing-project/
├── src/                    # Code hiện có
├── package.json            # Dependencies
├── ...                     # Các files khác
│
│ (Sau khi chạy install.sh)
│
├── .agent/                 ✅ ADDED
│   ├── workflows/
│   ├── roles/
│   └── project.json
├── docs/templates/         ✅ ADDED
└── plans/                  ✅ ADDED
```

**Không ghi đè files hiện có!**

### Trường hợp 2: DỰ ÁN MỚI

```
📁 new-project/
│
│ (Sau khi chạy install.sh)
│
├── .agent/                 ✅ ADDED
├── docs/templates/         ✅ ADDED
├── plans/                  ✅ ADDED
└── .gitignore              ✅ ADDED
```

---

## 🎯 Bắt Đầu Sử Dụng

### Option 1: Full Pipeline (Recommended cho feature mới)
```
/cook Implement user authentication with JWT
```
→ AI tự động chạy qua: PM → Architect → Engineer → QA → Reviewer

### Option 2: Từng Bước (Recommended cho kiểm soát)
```
/plan Create login feature    # Tạo PRD
/design                       # Thiết kế
/code                         # Implement
/test                         # Test
/review                       # Review
/git                          # Commit
```

### Option 3: Fix Bug
```
/fix Login button not working
```

---

## 📂 Cấu Trúc Sau Setup

```
.agent/
├── README.md              # Hướng dẫn chi tiết
├── project.json           # Config dự án
├── install.sh               # Script này
├── workflows/             # 10 workflows
│   ├── company-sop.md    # Master SOP
│   ├── init.md           # Khởi tạo
│   ├── plan.md           # Tạo PRD
│   ├── design.md         # Thiết kế
│   ├── code.md           # Implement
│   ├── cook.md           # Full pipeline
│   ├── fix.md            # Fix bugs
│   ├── test.md           # Testing
│   ├── review.md         # Code review
│   └── git.md            # Git operations
└── roles/                 # 7 roles
    ├── product-manager.md
    ├── architect.md
    ├── engineer.md
    ├── qa-engineer.md
    ├── code-reviewer.md
    ├── researcher.md
    └── devops.md

docs/templates/
├── code-standards.md      # Coding standards
├── prd-template.md        # PRD template
└── design-template.md     # Design template

plans/
├── active/               # Work in progress
├── reports/              # QA, review reports
├── archive/              # Completed plans
└── templates/            # Plan templates
```

---

## ❓ FAQ

### Q: Có cần cài đặt gì không?
**A:** Không. Chỉ cần Bash shell (có sẵn trên Mac/Linux, Windows dùng Git Bash).

### Q: Có ghi đè files của tôi không?
**A:** Không. Script chỉ thêm files mới, không ghi đè.

### Q: Dùng với AI Agent nào?
**A:** Compatible với Claude Code, Gemini CLI, hoặc bất kỳ AI agent nào hỗ trợ slash commands.

### Q: Có thể customize được không?
**A:** Có! Chỉnh sửa files trong `.agent/workflows/` và `.agent/roles/`.

---

## 🆘 Troubleshooting

### "Permission denied"
```bash
chmod +x install.sh
```

### "Command not found: bash"
Dùng Git Bash trên Windows hoặc WSL.

### Muốn chạy lại setup
```bash
rm -rf .agent
./install.sh
```

---

## 📞 Support

- Xem hướng dẫn đầy đủ: `.agent/README.md`
- Xem workflow cụ thể: `.agent/workflows/{name}.md`
- Xem role cụ thể: `.agent/roles/{name}.md`
