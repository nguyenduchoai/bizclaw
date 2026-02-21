# Hooks System - Comprehensive Guide

Event-driven automation framework for Claude Code with deterministic shell commands and LLM evaluations.

## Quick Reference

**Hook Types**: Command-based (bash) | Prompt-based (LLM evaluation)
**Configuration**: `.claude/settings.json` (4-tier hierarchy)
**Events**: 10 lifecycle triggers
**Matchers**: Exact, regex, wildcard patterns
**Security**: User assumes full responsibility

## Hook Events (10 Total)

| Event | Trigger Point | Common Use Cases |
|-------|---------------|------------------|
| **PreToolUse** | Before tool processing | Validation, blocking operations |
| **PermissionRequest** | Permission dialog display | Custom permission logic |
| **PostToolUse** | After tool completion | Post-processing, auto-formatting, logging |
| **Notification** | Notification sent | Desktop alerts, custom notifications |
| **UserPromptSubmit** | User prompt submission | Input validation, filtering |
| **Stop** | Main agent response complete | Session cleanup, logging |
| **SubagentStop** | Subagent completion | Subagent monitoring |
| **PreCompact** | Before compaction | Pre-compaction validation |
| **SessionStart** | Session init/resume | Environment setup |
| **SessionEnd** | Session termination | Cleanup operations |

## Configuration Structure

### Settings Hierarchy (Priority Order)

1. **Enterprise managed**: `managed-settings.json` (IT-deployed, cannot override)
2. **User settings**: `~/.claude/settings.json` (global defaults)
3. **Project settings**: `.claude/settings.json` (team-shared, version controlled)
4. **Local settings**: `.claude/settings.local.json` (personal, git-ignored)

### Basic Configuration

```json
{
  "hooks": {
    "EventName": [
      {
        "matcher": "ToolPattern",
        "hooks": [
          {
            "type": "command",
            "command": "bash script command"
          }
        ]
      }
    ]
  }
}
```

## Tool Matchers

**Exact matching**:
```json
"matcher": "Bash"
```

**Regex patterns**:
```json
"matcher": "Edit|Write"
```

**Wildcard (all tools)**:
```json
"matcher": "*"
```

**MCP tools**:
```json
"matcher": "mcp__servername__toolname"
```

**Built-in tools**: Write, Edit, Read, Bash, Task, Grep, Glob, WebFetch, WebSearch, NotebookEdit, SlashCommand, Skill

## Hook Types

### 1. Command-Based Hooks

Execute bash scripts with full shell capabilities.

**Configuration**:
```json
{
  "type": "command",
  "command": "jq -r '.tool_parameters.command' | tee -a .claude/commands.log"
}
```

**Input**: JSON via stdin with fields:
- `session_id`: Current session identifier
- `transcript_path`: Path to session transcript
- `cwd`: Current working directory
- `permission_mode`: Current permission setting
- `hook_event_name`: Triggering event name
- Event-specific fields (varies by hook)

**Output Control**:
- Exit code 0: Success, continue execution
- Exit code 2: Blocking error, halt execution
- Optional JSON output:
  ```json
  {
    "continue": false,
    "stopReason": "Reason for blocking",
    "decision": {}
  }
  ```

### 2. Prompt-Based Hooks

Leverage LLM evaluation for context-aware decisions.

**Supported Events**: Stop, SubagentStop only

**Configuration**:
```json
{
  "type": "prompt",
  "prompt": "Analyze the session transcript and determine if...",
  "model": "claude-3-5-sonnet-20241022"
}
```

**Output**: Structured JSON response based on LLM evaluation

## Common Use Cases

### Auto-Formatting (PostToolUse)

Automatically format code after edits.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "FILE=$(echo \"$INPUT\" | jq -r '.tool_parameters.file_path'); case \"$FILE\" in *.ts|*.js) npx prettier --write \"$FILE\" ;; *.go) gofmt -w \"$FILE\" ;; esac"
          }
        ]
      }
    ]
  }
}
```

**Script version** (`scripts/format-code.sh`):
```bash
#!/bin/bash
FILE=$(jq -r '.tool_parameters.file_path')

case "$FILE" in
  *.ts|*.tsx|*.js|*.jsx)
    npx prettier --write "$FILE"
    ;;
  *.go)
    gofmt -w "$FILE"
    ;;
  *.py)
    black "$FILE"
    ;;
esac
```

### File Protection (PreToolUse)

Block modifications to sensitive files.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "FILE=$(echo \"$INPUT\" | jq -r '.tool_parameters.file_path'); case \"$FILE\" in .env|*package-lock.json|.git/*) echo '{\"continue\":false,\"stopReason\":\"Protected file\"}'; exit 2 ;; esac"
          }
        ]
      }
    ]
  }
}
```

### Desktop Notifications (Notification)

Custom alerts when Claude needs input.

```json
{
  "hooks": {
    "Notification": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "MESSAGE=$(jq -r '.message'); notify-send 'Claude Code' \"$MESSAGE\""
          }
        ]
      }
    ]
  }
}
```

### Command Logger (PreToolUse)

Track bash commands for compliance.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '.tool_parameters | \"\\(.command) - \\(.description)\"' >> .claude/commands.log"
          }
        ]
      }
    ]
  }
}
```

### Markdown Enhancement (PostToolUse)

Auto-detect code block languages.

```bash
#!/usr/bin/env python3
# scripts/enhance-markdown.py
import json, sys, re

data = json.load(sys.stdin)
file_path = data['tool_parameters']['file_path']

if not file_path.endswith('.md'):
    sys.exit(0)

with open(file_path, 'r') as f:
    content = f.read()

# Detect unlabeled code blocks and add language tags
enhanced = re.sub(
    r'```\n((?:import|function|const|let|var|class)\b.*?)```',
    r'```javascript\n\1```',
    content,
    flags=re.DOTALL
)

with open(file_path, 'w') as f:
    f.write(enhanced)
```

**Configuration**:
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "python3 scripts/enhance-markdown.py"
          }
        ]
      }
    ]
  }
}
```

## Environment Variables

**Available in all hooks**:
- `$CLAUDE_PROJECT_DIR`: Project root path
- `${CLAUDE_PLUGIN_ROOT}`: Plugin directory path

**SessionStart only**:
- `CLAUDE_ENV_FILE`: Persist environment variables across session

**Hook input via stdin**: All data passed as JSON (parse with `jq`)

## Security Framework

### Critical Considerations

- Users assume **full responsibility** for hook commands
- Hooks execute with **user's environment credentials**
- Malicious hooks can **exfiltrate data**

### Best Practices

**Input Validation**:
```bash
# Validate file paths
FILE=$(jq -r '.tool_parameters.file_path')
if [[ ! "$FILE" =~ ^[a-zA-Z0-9/_.-]+$ ]]; then
  echo '{"continue":false,"stopReason":"Invalid path"}' && exit 2
fi
```

**Proper Quoting**:
```bash
# Always quote variables
FILE=$(jq -r '.tool_parameters.file_path')
prettier --write "$FILE"  # Correct
prettier --write $FILE    # Wrong - vulnerable to injection
```

**Path Traversal Prevention**:
```bash
# Use absolute paths and validate
REALPATH=$(realpath "$FILE")
if [[ ! "$REALPATH" =~ ^"$CLAUDE_PROJECT_DIR" ]]; then
  echo '{"continue":false,"stopReason":"Path outside project"}' && exit 2
fi
```

**Command Whitelisting**:
```bash
# Whitelist allowed commands
COMMAND=$(jq -r '.tool_parameters.command')
case "$COMMAND" in
  npm\ install|npm\ test|npm\ run\ *)
    # Allowed
    ;;
  *)
    echo '{"continue":false,"stopReason":"Command not allowed"}' && exit 2
    ;;
esac
```

## Hook Management

### Setup via /hooks Command

```bash
claude
> /hooks
```

Interactive setup wizard for hook configuration.

### Manual Configuration

Edit `.claude/settings.json` or `.claude/settings.local.json`:

```json
{
  "hooks": {
    "PreToolUse": [...],
    "PostToolUse": [...]
  }
}
```

### Testing Hooks

```bash
# Enable debug mode
claude --debug

# Test hook with sample input
echo '{"session_id":"test","tool_parameters":{"command":"ls"}}' | bash scripts/test-hook.sh
```

### Debugging

**Check logs**:
```bash
cat .claude/logs/hooks.log
```

**Common issues**:
- Verify script permissions: `chmod +x scripts/*.sh`
- Check JSON syntax in settings
- Validate script paths (relative to project root)
- Test stdin parsing: `echo '{}' | jq`

## Plugin Integration

Hooks can be bundled in plugins for distribution.

**Plugin structure**:
```
my-plugin/
├── plugin.json
├── hooks/
│   └── settings.json  # Hook configurations
└── scripts/
    └── hook-script.sh
```

**plugin.json**:
```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "Plugin with hooks",
  "hooks": "hooks/settings.json"
}
```

**hooks/settings.json**:
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/hook-script.sh"
          }
        ]
      }
    ]
  }
}
```

## Advanced Patterns

### Conditional Execution

```bash
#!/bin/bash
# Only run in production branch
BRANCH=$(git branch --show-current)
if [[ "$BRANCH" == "main" ]]; then
  # Execute hook logic
  echo "Running on production"
fi
```

### Multi-step Processing

```bash
#!/bin/bash
FILE=$(jq -r '.tool_parameters.file_path')

# Step 1: Format
prettier --write "$FILE"

# Step 2: Lint
eslint --fix "$FILE"

# Step 3: Test
npm test -- "$FILE"
```

### Notification Integration

```bash
#!/bin/bash
# Slack webhook notification
MESSAGE=$(jq -r '.message')
curl -X POST "$SLACK_WEBHOOK_URL" \
  -H 'Content-Type: application/json' \
  -d "{\"text\":\"$MESSAGE\"}"
```

### Error Handling

```bash
#!/bin/bash
set -euo pipefail

FILE=$(jq -r '.tool_parameters.file_path')

# Validate file exists
if [[ ! -f "$FILE" ]]; then
  echo '{"continue":false,"stopReason":"File not found"}' >&2
  exit 2
fi

# Process file
prettier --write "$FILE" || {
  echo '{"continue":false,"stopReason":"Format failed"}' >&2
  exit 2
}
```

## Performance Considerations

**Keep hooks fast**: <100ms execution time ideal

**Async operations**:
```bash
#!/bin/bash
# Run in background for long operations
{
  # Long-running task
  npm run build
} &
```

**Caching**:
```bash
#!/bin/bash
CACHE_FILE=".claude/cache/format-cache"
FILE_HASH=$(md5sum "$FILE" | cut -d' ' -f1)

# Check cache
if grep -q "$FILE_HASH" "$CACHE_FILE" 2>/dev/null; then
  echo "Using cached result" >&2
  exit 0
fi

# Process and cache
prettier --write "$FILE"
echo "$FILE_HASH" >> "$CACHE_FILE"
```

## Example: Complete Hook System

**File**: `.claude/settings.json`

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/scripts/validate-bash.sh"
          }
        ]
      },
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/scripts/protect-files.sh"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/scripts/auto-format.sh"
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/scripts/desktop-notify.sh"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/scripts/log-usage.sh"
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Review the session and log key accomplishments.",
            "model": "claude-3-5-sonnet-20241022"
          }
        ]
      }
    ]
  }
}
```

## Reference Links

- **Hooks Guide**: https://code.claude.com/docs/en/hooks-guide
- **Hooks API Reference**: https://code.claude.com/docs/en/hooks
- **Settings Configuration**: https://code.claude.com/docs/en/settings
- **Plugins Reference**: https://code.claude.com/docs/en/plugins-reference
- **GitHub Examples**: https://github.com/anthropics/claude-code/tree/main/examples/hooks

## See Also

- **Settings**: `references/configuration.md`
- **Plugins**: `references/hooks-and-plugins.md` (plugin-specific)
- **MCP Integration**: `references/mcp-integration.md`
- **Security Best Practices**: `references/best-practices.md`
