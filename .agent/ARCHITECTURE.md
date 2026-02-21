# Bizino AI DEV - Architecture Documentation

> Complete system architecture for the Bizino AI DEV Kit v3.2

## 📋 Overview

Bizino AI DEV is a modular AI Agent system that simulates a complete software company:

- **21 Specialist Agents** - Role-based AI personas
- **55+ Skills** - Domain-specific knowledge modules  
- **23 Workflows** - Slash command procedures
- **6 Rules** - Always-follow guidelines
- **20 MCP Servers** - External integrations

---

## 🏗️ Directory Structure

```
.agent/
├── ARCHITECTURE.md          # This file
├── agents/                  # 21 Specialist Agents
│   ├── brainstormer.md
│   ├── build-error-resolver.md    # NEW v3.2
│   ├── code-reviewer.md
│   ├── copywriter.md
│   ├── database-admin.md
│   ├── debugger.md
│   ├── docs-manager.md
│   ├── e2e-runner.md              # NEW v3.2
│   ├── fullstack-developer.md
│   ├── git-manager.md
│   ├── journal-writer.md
│   ├── mcp-manager.md
│   ├── planner.md
│   ├── project-manager.md
│   ├── refactor-cleaner.md        # NEW v3.2
│   ├── researcher.md
│   ├── scout-external.md
│   ├── scout.md
│   ├── security-reviewer.md       # NEW v3.2
│   ├── tester.md
│   └── ui-ux-designer.md
├── skills/                  # 55+ Skills
├── workflows/               # 23 Slash Commands
├── rules/                   # NEW v3.2 - Global Rules
│   ├── security.md
│   ├── coding-style.md
│   ├── testing.md
│   ├── git-workflow.md
│   ├── performance.md
│   └── patterns.md
├── hooks/                   # NEW v3.2 - Event Hooks
└── mcp-configs/             # NEW v3.2 - MCP Servers
    └── mcp-servers.json
```

---

## 🤖 Agents (21)

Specialist AI personas for different domains.

| Agent | Focus | Key Skills |
|-------|-------|------------|
| `brainstormer` | Ideation, exploration | brainstorming |
| `build-error-resolver` | Fix build/type errors | debugging, fixing |
| `code-reviewer` | Code quality review | code-review, clean-code |
| `copywriter` | Marketing copy | copywriting |
| `database-admin` | Database design | databases, planning |
| `debugger` | Root cause analysis | debugging, sequential-thinking |
| `docs-manager` | Documentation | planning |
| `e2e-runner` | E2E testing | tdd-workflow |
| `fullstack-developer` | Full-stack dev | frontend-development, backend-development |
| `git-manager` | Version control | git |
| `journal-writer` | Session notes | continuous-learning |
| `mcp-manager` | MCP operations | mcp-management |
| `planner` | Task planning | planning, problem-solving |
| `project-manager` | Project coordination | planning, parallel-agents |
| `refactor-cleaner` | Code cleanup | clean-code, refactor |
| `researcher` | Technical research | research, docs-seeker |
| `scout-external` | External research | research |
| `scout` | Internal codebase analysis | - |
| `security-reviewer` | Security audit | vulnerability-scanner, security |
| `tester` | Testing strategies | tdd-workflow, testing |
| `ui-ux-designer` | UI/UX design | ui-ux-pro-max, frontend-design |

---

## 🧠 Skills (55+)

Domain-specific knowledge modules. Skills are loaded on-demand based on task context.

### AI & Multimodal
| Skill | Description |
|-------|-------------|
| `ai-artist` | Prompt engineering for LLMs/Image/Video |
| `ai-multimodal` | Image/Audio/Video analysis with Gemini |
| `google-adk-python` | Build AI Agents with Google ADK |

### Development
| Skill | Description |
|-------|-------------|
| `frontend-design` | UI implementation patterns |
| `frontend-development` | React/Next.js development |
| `backend-development` | API, auth, database patterns |
| `web-frameworks` | Next.js, Turborepo, RemixIcon |
| `mobile-development` | React Native, Flutter |
| `react-best-practices` | 45 rules from Vercel Engineering |

### Code Quality (NEW v3.2)
| Skill | Description |
|-------|-------------|
| `coding-standards` | TypeScript/JavaScript standards |
| `clean-code` | Clean code principles, refactoring |
| `tdd-workflow` | Test-Driven Development |
| `vulnerability-scanner` | Security vulnerability detection |

### Design & Visualization
| Skill | Description |
|-------|-------------|
| `ui-styling` | shadcn/ui, Tailwind patterns |
| `ui-ux-pro-max` | 50 styles, 21 palettes, 50 fonts |
| `mermaidjs-v11` | Diagram generation |
| `threejs` | 3D visualizations |
| `web-design-guidelines` | Web Interface Guidelines |

### Infrastructure
| Skill | Description |
|-------|-------------|
| `devops` | Cloudflare, Docker, GCP |
| `databases` | PostgreSQL, MongoDB |
| `payment-integration` | SePay, Polar |

### Tools & Utilities
| Skill | Description |
|-------|-------------|
| `debugging` | Systematic debugging |
| `fixing` | Unified bug fixing with complexity routing |
| `code-review` | Review practices |
| `research` | Technical research patterns |
| `brainstorming` | Solution ideation |
| `context-engineering` | Context optimization |
| `sequential-thinking` | Complex analysis |
| `problem-solving` | Advanced techniques |
| `git` | Git workflows, conventional commits |
| `copywriting` | Marketing/technical copy |

### Agent Coordination (NEW v3.2)
| Skill | Description |
|-------|-------------|
| `parallel-agents` | Multi-agent patterns |
| `continuous-learning` | Session learning, pattern extraction |

### Project Management
| Skill | Description |
|-------|-------------|
| `plans-kanban` | Visual plans dashboard |
| `planning` | Technical planning |
| `repomix` | Codebase packaging |

---

## 🔄 Workflows (23)

Slash command procedures for common tasks.

### Core Development
| Command | Description |
|---------|-------------|
| `/cook` | 🔥 Full Auto Pipeline - Idea → MVP |
| `/plan` | 📋 Create PRD from requirements |
| `/design` | 🏗️ System design from PRD |
| `/code` | 💻 Implement code from design |
| `/test` | 🧪 Run tests and generate reports |
| `/review` | 👀 Code review and quality check |

### Operations
| Command | Description |
|---------|-------------|
| `/run` | ▶️ Smart App Launcher |
| `/fix` | 🔧 Unified Bug Fixing |
| `/debug` | 🐞 Sherlock Mode debugging |
| `/audit` | 🏥 Code health check |
| `/refactor` | 🧹 Safe Code Cleanup |
| `/deploy` | 🚀 Deploy to production |
| `/rollback` | ⏪ Emergency Recovery |
| `/kanban` | 📊 Visual Plans Dashboard |

### Knowledge Management
| Command | Description |
|---------|-------------|
| `/recap` | 🧠 Memory Retriever |
| `/save-brain` | 💾 Infinite Memory |
| `/visualize` | 🎨 UI/UX Design Partner |

### Utility
| Command | Description |
|---------|-------------|
| `/init` | 🚀 Init project structure |
| `/git` | 📦 Git operations |

---

## 📏 Rules System (NEW v3.2)

Global rules that ALWAYS apply to every task.

| Rule | Purpose |
|------|---------|
| `security.md` | Secrets, XSS, injection prevention, OWASP |
| `coding-style.md` | File organization, naming, immutability |
| `testing.md` | TDD workflow, coverage requirements |
| `git-workflow.md` | Conventional commits, branch strategy |
| `performance.md` | Model selection, context management |
| `patterns.md` | API responses, error handling |

---

## 🔗 MCP Configurations (NEW v3.2)

Pre-configured MCP servers for external integrations.

| Category | Servers |
|----------|---------|
| **Essential** | github, memory, context7 |
| **Database** | supabase, postgres, clickhouse |
| **Deployment** | vercel, railway, cloudflare-* |
| **Development** | puppeteer, firecrawl, brave-search |
| **Productivity** | linear, sentry |

---

## 🎯 Skill Loading Protocol

### When Skills Load
1. User mentions skill keywords
2. Workflow requires specific skill
3. Agent delegates to skill
4. Task context matches skill description

### Skill Structure
```markdown
---
name: skill-name
description: When to use this skill
---

# Skill Content

## When to Activate
...

## Instructions
...

## Patterns
...
```

### Enhanced Skills (with resources)
```
skill-name/
├── SKILL.md           # Main instructions
├── scripts/           # Helper scripts
├── references/        # Reference docs
└── examples/          # Usage examples
```

---

## 📊 Statistics

| Category | Count | Change |
|----------|-------|--------|
| Agents | 21 | +4 from v3.1 |
| Skills | 55+ | +8 from v3.1 |
| Workflows | 23 | Same |
| Rules | 6 | NEW |
| MCP Servers | 20 | NEW |

---

## 🔗 Quick Reference

### Daily Workflow
```
Morning:   /recap → /audit → /run
Develop:   /plan → /design → /code → /test → /review
End of Day: /save-brain → /git
```

### Problem Solving
```
Bug:       /fix or /debug
Build:     Use build-error-resolver agent
Security:  Use security-reviewer agent
Refactor:  /refactor
```

### Multi-Agent Tasks
```
Feature:   Planner → Coder → Tester → Reviewer
Audit:     Security + Performance + Accessibility (parallel)
Research:  Scout + Researcher + Docs-Seeker
```

---

## 🔄 Version History

### v3.2.0 (2026-01-21)
- Added Rules System (6 rules)
- Added MCP Configs (20 servers)
- Added 4 new agents (build-error-resolver, security-reviewer, e2e-runner, refactor-cleaner)
- Added 8 new skills (tdd-workflow, coding-standards, clean-code, vulnerability-scanner, parallel-agents, continuous-learning)
- Added ARCHITECTURE.md

### v3.1.0 (2026-01-18)
- Added react-best-practices
- Added plans-kanban
- Added fixing skill
- Added /kanban workflow

---

*Bizino AI DEV v3.2 - "We engineer solutions, not just code."*
