---
description: ⏪ Quay lại phiên bản cũ - The Time Machine
---

# WORKFLOW: /rollback - Emergency Recovery System

> **Role**: Antigravity Emergency Responder  
> **Mission**: Giúp user "quay về quá khứ" khi code hỏng
> **Principle**: "Calm & Calculated - Bình tĩnh, không hoảng loạn"

// turbo-all

---

## 🎯 Mục đích

Workflow này giúp:
- Nhanh chóng khôi phục từ deployment thất bại
- Revert code changes gây lỗi
- Restore database nếu cần
- Giữ bình tĩnh trong tình huống khẩn cấp

---

## Phase 1: Damage Assessment 🔍

### 1.1 Quick Triage

```markdown
"Bình tĩnh, em sẽ giúp anh sửa."

"Anh vừa làm gì mà nó hỏng?"

A) 💻 Sửa code (file nào?)
B) 🚀 Vừa deploy
C) 📦 Cài thêm package
D) 🗄️ Chạy migration/seed
E) ❓ Không biết, tự nhiên hỏng
```

### 1.2 Severity Check

```markdown
"Nó hỏng kiểu gì?"

┌─────────────────────────────────────────────────┐
│ 🔴 CRITICAL: Không mở được app, trắng toàn bộ  │
│    → Priority: IMMEDIATE ROLLBACK               │
│                                                  │
│ 🟡 HIGH: Mở được nhưng lỗi chức năng chính     │
│    → Priority: Quick fix hoặc rollback          │
│                                                  │
│ 🟢 MEDIUM: Chỉ lỗi một phần nhỏ               │
│    → Priority: Debug, không cần rollback        │
│                                                  │
│ ⚪ LOW: UI lệch, không ảnh hưởng chức năng     │
│    → Priority: Hotfix, tiếp tục debug           │
└─────────────────────────────────────────────────┘
```

### 1.3 Auto-Scan Recent Changes

```bash
# Xem files thay đổi gần đây
git log --oneline -5
git diff HEAD~1 --stat

# Xem uncommitted changes
git status

# Xem stashed changes
git stash list
```

---

## Phase 2: Recovery Options 🔄

### 2.1 Code Rollback Options

```markdown
Các phương án khôi phục:

┌─────────────────────────────────────────────────┐
│ A) 📄 Rollback 1 file cụ thể                    │
│    → An toàn nhất, giữ nguyên code khác         │
│    → git checkout HEAD~1 -- path/to/file        │
│                                                  │
│ B) 📝 Rollback 1 commit gần nhất               │
│    → Hoàn tác commit cuối                       │
│    → git revert HEAD                            │
│                                                  │
│ C) 📁 Rollback toàn bộ uncommitted changes     │
│    → Reset về trạng thái clean                  │
│    → git checkout .                             │
│                                                  │
│ D) ⏰ Rollback về commit cụ thể                 │
│    → Quay về thời điểm chọn                     │
│    → git reset --hard <commit-hash>             │
│                                                  │
│ E) 🔧 Không rollback, sửa thủ công             │
│    → Giữ code mới, debug và fix                 │
│    → Chuyển sang /debug                         │
└─────────────────────────────────────────────────┘
```

### 2.2 Deployment Rollback Options

```markdown
Nếu vừa deploy lỗi:

┌─────────────────────────────────────────────────┐
│ Vercel:                                          │
│ → Dashboard → Deployments → Promote previous    │
│                                                  │
│ Railway:                                         │
│ → Dashboard → Deployments → Rollback            │
│                                                  │
│ Docker/K8s:                                      │
│ → kubectl rollout undo deployment/app           │
│                                                  │
│ Traditional VPS:                                 │
│ → ssh server                                    │
│ → cd /app && git checkout <previous-tag>        │
│ → npm run build && pm2 restart                  │
└─────────────────────────────────────────────────┘
```

### 2.3 Database Rollback Options

```markdown
⚠️ NGUY HIỂM - Chỉ khi cần thiết:

┌─────────────────────────────────────────────────┐
│ A) Rollback migration                           │
│    → npm run migrate:rollback                   │
│                                                  │
│ B) Restore từ backup                            │
│    → pg_restore -d database backup.sql          │
│                                                  │
│ C) Point-in-time recovery (nếu có)             │
│    → Yêu cầu managed database                   │
└─────────────────────────────────────────────────┘
```

---

## Phase 3: Execution 🔧

### 3.1 Rollback Single File

```bash
# Xem file trước khi rollback
git show HEAD~1:path/to/file

# Rollback file cụ thể
git checkout HEAD~1 -- path/to/file

# Verify
git diff HEAD -- path/to/file
```

### 3.2 Rollback Last Commit

```bash
# Tạo commit revert (safe, keeps history)
git revert HEAD --no-edit

# Hoặc reset (risky, removes history)
git reset --soft HEAD~1  # Keep changes staged
git reset --hard HEAD~1  # Discard changes
```

### 3.3 Rollback All Uncommitted

```bash
# Stash first (backup)
git stash push -m "backup before rollback"

# Then reset
git checkout .
git clean -fd

# Verify clean state
git status
```

### 3.4 Rollback to Specific Point

```bash
# Find the good commit
git log --oneline -20

# Create backup branch
git branch backup-current-state

# Reset to good commit
git reset --hard abc1234

# Force push (if needed)
git push --force-with-lease
```

---

## Phase 4: Verification ✅

### 4.1 Post-Rollback Checks

```bash
# Verify git status
git status
git log -1

# Verify build
npm run build

# Verify tests
npm test

# Verify app runs
npm run dev
```

### 4.2 Confirmation

```markdown
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⏪ ROLLBACK COMPLETE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Method: [Rollback type used]
Target: [Commit hash or file]
Time: [Timestamp]

Verification:
✅ Git status clean
✅ Build successful
✅ Tests passing
✅ App running

"App đã về trạng thái trước khi lỗi."
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## Phase 5: Post-Recovery 🔒

### 5.1 Document What Happened

```markdown
📋 Incident Report:

**What broke:** [Description]
**When:** [Timestamp]
**Cause:** [Root cause]
**Solution:** [How it was fixed]
**Prevention:** [How to prevent next time]
```

### 5.2 Prevention Tips

```markdown
"Lần sau trước khi sửa lớn:"

1. 📌 Tạo backup branch: git checkout -b backup/feature-x
2. ✅ Chạy tests trước khi commit
3. 🔍 Review changes: git diff
4. 💾 Commit thường xuyên (small commits)
5. 📝 Document những gì đang làm
```

---

## Emergency Commands Cheat Sheet 🆘

```bash
# ═══════════════════════════════════════════════
# EMERGENCY ROLLBACK COMMANDS
# ═══════════════════════════════════════════════

# Reset uncommitted changes
git checkout .
git clean -fd

# Undo last commit (keep files)
git reset --soft HEAD~1

# Undo last commit (discard files)  
git reset --hard HEAD~1

# Rollback specific file
git checkout HEAD~1 -- file.ts

# View before rollback
git show HEAD~1:file.ts

# Create revert commit
git revert HEAD --no-edit

# Emergency backup
git stash push -m "emergency backup"

# Restore from stash
git stash pop
```

---

## ⚠️ NEXT STEPS

```
1️⃣ Rollback xong? → /run để test lại app
2️⃣ Muốn sửa thay vì rollback? → /debug
3️⃣ Cần review code? → /review
4️⃣ OK rồi? → /save-brain để document incident
5️⃣ Deploy lại? → /deploy
```

---

## 💡 PREVENTION RULES

| Rule | Description |
|------|-------------|
| Small commits | Dễ rollback hơn |
| Test before push | Catch errors early |
| Feature branches | Isolate changes |
| Backup before big changes | Safety net |
| Document what you do | Easier debugging |

---

## 🚫 DO NOT

```markdown
❌ Panic và sửa random
❌ Force push main branch without backup
❌ Delete git history
❌ Ignore the root cause
❌ Skip documentation after fixing
```

---

*"The best debugging session is the one you don't need to have."*
