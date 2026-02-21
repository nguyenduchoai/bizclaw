# ⚡ Bizino AI DEV - Antigravity / Gemini Kit

> Software Company Agent System Kit for **Antigravity IDE** and **Google Gemini**

## 📦 Quick Install

```bash
# From this directory
./install.sh [target_project_directory]

# Or from root
../install.sh --antigravity [target_project_directory]
```

## 📁 What Gets Installed

```
your-project/
├── GEMINI.md                    # Main configuration file
├── .agent/
│   ├── workflows/               # Automated workflow definitions
│   │   ├── cook.md              # 🔥 Full auto pipeline
│   │   ├── plan.md              # Create PRD
│   │   ├── design.md            # System design
│   │   ├── code.md              # Implementation
│   │   ├── test.md              # Testing
│   │   ├── review.md            # Code review
│   │   ├── fix.md               # Bug fixing
│   │   ├── git.md               # Git operations
│   │   └── init.md              # Project initialization
│   └── roles/                   # AI role definitions
│       ├── product-manager.md
│       ├── architect.md
│       ├── engineer.md
│       ├── qa-engineer.md
│       ├── code-reviewer.md
│       ├── researcher.md
│       └── devops.md
├── plans/                       # Project documentation
│   ├── active/
│   ├── reports/
│   └── archive/
└── docs/
    └── templates/
```

## 🚀 Usage

### Slash Commands

Use these commands in Antigravity/Gemini:

| Command | Description |
|---------|-------------|
| `/cook [request]` | 🔥 Full auto pipeline - from idea to MVP |
| `/plan [feature]` | Create PRD for a feature |
| `/design [prd]` | Create system design from PRD |
| `/code [design]` | Implement code from design |
| `/test [code]` | Run tests and generate reports |
| `/review [code]` | Code review and quality check |
| `/fix [issue]` | Debug and fix issues |
| `/git [action]` | Git operations (commit, push, PR) |

### Example Workflows

```
# Build a complete application
/cook Build an e-commerce app with user authentication

# Plan a new feature
/plan User authentication with OAuth2 and social login

# Fix a bug
/fix The login form is not validating email format
```

## 🎭 Roles

The kit includes 7 specialized AI roles:

1. **Product Manager** - Requirements analysis, PRD creation
2. **Architect** - System design, technical decisions
3. **Engineer** - Code implementation
4. **QA Engineer** - Testing, quality assurance
5. **Code Reviewer** - Code review, best practices
6. **Researcher** - Technical research, documentation
7. **DevOps** - Deployment, infrastructure

## ⚙️ Configuration

Edit `GEMINI.md` to customize:
- System identity and behavior
- Auto-run rules
- Output locations
- UI framework preferences
- Core principles

## 📚 Documentation

- [GETTING_STARTED.md](./GETTING_STARTED.md) - Quick start guide
- [workflows/](./workflows/) - Detailed workflow documentation
- [roles/](./roles/) - Role specifications

---

**Bizino AI DEV** - *Transforming Ideas into Software Automatically*
