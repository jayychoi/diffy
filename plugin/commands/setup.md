---
description: Set up diffy with Claude Code hook integration
---

# Diffy Setup

Install diffy binary and configure the Claude Code hook.

## Installation

### Option 1: Via Homebrew (Recommended)
```bash
brew tap jayychoi/diffy
brew install diffy
```

### Option 2: Via Cargo
```bash
cargo install --git https://github.com/jayychoi/diffy diffy-tui
```

### Option 3: Download Binary
Visit [Releases](https://github.com/jayychoi/diffy/releases) and download for your platform.

## Verification

```bash
diffy --version
```

## Configuration (Optional)

Create `~/.config/diffy/config.toml`:

```toml
[ui]
theme = "dark"
mouse = true

[feedback]
max_size = 10240  # bytes
```

## How It Works

The diffy plugin includes a **Stop hook** that auto-runs diffy after Claude Code edits:

1. Claude generates/modifies code
2. Stop hook triggers → `diffy --hook-mode --apply`
3. You review hunks in the TUI (accept/reject)
4. Rejected hunks auto-revert
5. Feedback sent to Claude → Claude refines code

No additional setup needed! The hook is configured automatically.

## Workflow

```
[Claude generates code]
         ↓
[Stop hook runs diffy]
         ↓
[You review hunks in TUI]
         ↓
[Accept/reject hunks]
         ↓
[Rejected hunks revert + feedback sent]
         ↓
[Claude reads feedback and fixes]
```

## Troubleshooting

**Diffy not found?**
```bash
which diffy
diffy --version
```

**Hook not running?**
- Check plugin is enabled: `/plugin` → manage diffy → enable hooks
- Debug: `claude --debug` shows hook execution

**Unstaged changes not showing?**
- Ensure you're in a git repository
- Check: `git status`

For more help: `diffy --help`
