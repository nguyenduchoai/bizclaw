---
description: Git workflow chuẩn - commit, push, PR
---

# /git - Git Workflow

## Available Commands

| Command | Description |
|---------|-------------|
| `/git:status` | Check git status |
| `/git:commit` | Stage all & commit |
| `/git:push` | Commit & push |
| `/git:pr` | Create pull request |

---

## /git:status
// turbo
```bash
git status
git log --oneline -5
```

---

## /git:commit

### Step 1: Check Status
// turbo
```bash
git status
git diff --stat
```

### Step 2: Review Changes
- Review what's being committed
- Ensure no sensitive data

### Step 3: Stage & Commit
```bash
git add .
git commit -m "{type}: {description}"
```

### Commit Message Format
```
{type}({scope}): {subject}

{body}

{footer}
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code refactoring
- `test`: Tests
- `chore`: Maintenance

**Examples:**
```
feat(auth): add user authentication
fix(api): handle null response from payment gateway
docs(readme): update installation instructions
```

---

## /git:push

### Step 1: Pre-push Checks
// turbo
```bash
# Run tests before push
npm test 2>/dev/null || php artisan test 2>/dev/null || pytest 2>/dev/null || echo "No test command"
```

### Step 2: Push
```bash
git push origin $(git branch --show-current)
```

### Step 3: Output
```
📤 Pushed to remote

Branch: {branch}
Commits: {count}
Remote: {url}
```

---

## /git:pr

### Step 1: Create PR Title
Format: `{type}: {description}`

### Step 2: PR Description Template
```markdown
## Description
{What this PR does}

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Manual testing done

## Checklist
- [ ] Code follows project style
- [ ] Self-reviewed
- [ ] No console.log/debug code
- [ ] Documentation updated
```

### Step 3: Create PR
```bash
gh pr create --title "{title}" --body "{body}"
```

---

## Pre-commit Rules

Before any commit:
- [ ] Run linting
- [ ] Run tests
- [ ] No sensitive data (API keys, passwords)
- [ ] Clean commit message (no "WIP", "fix fix")
- [ ] No AI attribution needed

## Branch Naming

```
{type}/{ticket-id}-{description}

Examples:
- feat/ABC-123-user-authentication
- fix/ABC-456-payment-bug
- docs/ABC-789-api-documentation
```
