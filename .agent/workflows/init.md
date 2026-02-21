---
description: Khởi tạo Bizino AI DEV cho dự án mới
---

# /init - Initialize Bizino AI DEV

## 🎯 Mục Đích
Khởi tạo hệ thống Bizino AI DEV cho dự án, giúp AI hiểu context và sẵn sàng phát triển.

## ⚡ Workflow
// turbo-all

### Step 1: Load System Configuration
```bash
# Đọc GEMINI.md để hiểu system rules
cat GEMINI.md
```

### Step 2: Analyze Project
// turbo
```bash
# Xem cấu trúc project
ls -la
find . -maxdepth 2 -type f -name "*.json" 2>/dev/null | head -5
find . -maxdepth 2 -type f -name "*.md" 2>/dev/null | head -10
```

### Step 3: Detect Project Type

**Auto-detect:**
- `package.json` → Node.js/TypeScript
- `composer.json` → PHP/Laravel
- `requirements.txt` → Python
- `go.mod` → Go
- `Cargo.toml` → Rust

### Step 4: Create/Update Project Config

Tạo `.agent/project.json`:
```json
{
  "name": "{detected-name}",
  "type": "{detected-type}",
  "language": "{detected-language}",
  "framework": "{detected-framework}",
  "commands": {
    "test": "{test-command}",
    "lint": "{lint-command}",
    "build": "{build-command}",
    "dev": "{dev-command}"
  },
  "autoRun": {
    "tests": true,
    "lint": true,
    "deploy": false
  },
  "initialized": "{current-date}"
}
```

### Step 5: Output

```
╔═══════════════════════════════════════════════════════════════════╗
║      🚀 BIZINO AI DEV - INITIALIZED                               ║
╠═══════════════════════════════════════════════════════════════════╣
║                                                                    ║
║  📁 Project: {name}                                                ║
║  💻 Language: {language}                                           ║
║  🔧 Framework: {framework}                                         ║
║                                                                    ║
╠═══════════════════════════════════════════════════════════════════╣
║  📋 AVAILABLE COMMANDS                                             ║
║                                                                    ║
║  🔥 /cook [request]  - Full Auto: Request → MVP                   ║
║  📝 /plan [feature]  - Create PRD                                  ║
║  🏗️ /design          - System Design                               ║
║  👨‍💻 /code            - Implement Code                              ║
║  🧪 /test            - Run Tests                                   ║
║  👁️ /review          - Code Review                                 ║
║  🔧 /fix [issue]     - Fix Bugs                                    ║
║  📤 /git             - Git Operations                              ║
║                                                                    ║
╠═══════════════════════════════════════════════════════════════════╣
║  💡 QUICK START                                                    ║
║                                                                    ║
║  Just tell me what you want to build:                              ║
║  "Build an e-commerce app with user auth and payment"              ║
║                                                                    ║
║  Or use explicit command:                                          ║
║  /cook Build a task management application                         ║
║                                                                    ║
╚═══════════════════════════════════════════════════════════════════╝

🎯 Ready! What would you like me to build?
```

---

## 🔄 Auto-Trigger

Workflow này tự động chạy khi:
1. User lần đầu mở conversation trong project
2. User nói "init", "start", "begin", "khởi tạo"
3. Không có `.agent/project.json`

---

## 📝 Notes

- Nếu project đã có `.agent/project.json`, chỉ hiển thị Quick Start
- Nếu project mới, tạo đầy đủ cấu trúc

---

**Bizino AI DEV** - *Transforming Ideas into Software Automatically*
