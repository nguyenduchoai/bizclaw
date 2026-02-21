# Git Workflow Rules - ALWAYS FOLLOW

> Conventional commits, clean history, and professional PR process.

## 📝 Commit Message Format

### Structure
```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

### Types
| Type | Description | Example |
|------|-------------|---------|
| `feat` | New feature | `feat(auth): add Google OAuth login` |
| `fix` | Bug fix | `fix(api): handle null response` |
| `docs` | Documentation | `docs(readme): update installation guide` |
| `style` | Formatting | `style: fix indentation` |
| `refactor` | Code change (no feature/fix) | `refactor(user): extract validation logic` |
| `perf` | Performance | `perf(query): add database index` |
| `test` | Tests | `test(auth): add login integration tests` |
| `chore` | Maintenance | `chore(deps): update dependencies` |
| `ci` | CI/CD | `ci: add GitHub Actions workflow` |
| `build` | Build system | `build: update webpack config` |

### Subject Rules
- Use imperative mood ("add" not "added")
- No period at end
- Max 50 characters
- Lowercase first letter

### Examples
```bash
# ✅ GOOD
feat(payment): add Stripe checkout integration
fix(auth): prevent session fixation attack
refactor(api): extract error handling middleware
docs(api): add OpenAPI specification

# ❌ BAD
Fixed bug                    # Not specific
feat: Added user login.      # Wrong tense, period
update                       # No context
WIP                          # Not meaningful
```

## 🌳 Branch Strategy

### Branch Naming
```
<type>/<ticket-id>-<short-description>

Examples:
feature/AUTH-123-google-oauth
fix/BUG-456-login-redirect
refactor/TECH-789-api-cleanup
```

### Branch Types
| Prefix | Purpose |
|--------|---------|
| `feature/` | New features |
| `fix/` | Bug fixes |
| `hotfix/` | Production emergency fixes |
| `refactor/` | Code improvements |
| `docs/` | Documentation |
| `test/` | Test additions |

### Main Branches
```
main        → Production-ready code
develop     → Integration branch (optional)
```

## 🔄 Pull Request Process

### Before Creating PR
```bash
# 1. Update from main
git fetch origin main
git rebase origin/main

# 2. Run tests
npm test

# 3. Run linter
npm run lint

# 4. Check build
npm run build
```

### PR Title Format
```
[TYPE] Short description (#ticket-id)

Examples:
[FEAT] Add user authentication (#AUTH-123)
[FIX] Resolve payment timeout issue (#BUG-456)
```

### PR Description Template
```markdown
## Summary
Brief description of changes.

## Changes
- Change 1
- Change 2

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing completed

## Screenshots (if UI changes)
[Attach screenshots]

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-reviewed changes
- [ ] Documentation updated
- [ ] No console.log statements
- [ ] No hardcoded secrets
```

## ⚠️ Pre-Push Checklist

### Automated Checks
```bash
# Run before every push
npm run lint          # No linter errors
npm run typecheck     # No TypeScript errors
npm test              # All tests pass
npm run build         # Build succeeds
```

### Manual Checks
- [ ] No `console.log` in production code
- [ ] No hardcoded secrets or API keys
- [ ] No commented-out code
- [ ] No TODO comments (unless tracked)
- [ ] Meaningful commit messages

## 🔙 Reverting Changes

### Revert Commit
```bash
# Revert specific commit
git revert <commit-hash>

# Revert merge commit
git revert -m 1 <merge-commit-hash>
```

### Reset (Use Carefully!)
```bash
# Soft reset (keep changes staged)
git reset --soft HEAD~1

# Hard reset (discard changes)
git reset --hard HEAD~1  # ⚠️ DANGER: Lost forever
```

## 📊 Git Hygiene

### Keep History Clean
```bash
# Squash before merging
git rebase -i HEAD~3  # Squash last 3 commits

# Amend last commit
git commit --amend

# Interactive rebase to clean up
git rebase -i origin/main
```

### Avoid
- Merging `main` into feature branch (rebase instead)
- Force pushing to shared branches
- Committing large binary files
- Committing node_modules or build artifacts

## 🔐 Security

### Never Commit
- `.env` files with real secrets
- API keys or passwords
- Private keys or certificates
- Personal access tokens

### Always Have
```gitignore
# .gitignore
.env
.env.local
.env.production
*.pem
*.key
```
