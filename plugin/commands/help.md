---
description: Show diffy usage and keyboard shortcuts
---

# Diffy Help

## What is Diffy?

An interactive TUI diff reviewer for Claude Code. Review code hunks one-by-one, accept/reject changes, and send structured feedback back to Claude.

## Basic Workflow

1. Claude Code generates code changes
2. Stop hook runs: `diffy --hook-mode --apply`
3. You review each hunk in the TUI
4. Rejected hunks auto-revert, feedback sent to Claude
5. Claude reads feedback and refines the code

## Keyboard Shortcuts

### Navigation
- `j` / `↓`: Next hunk
- `k` / `↑`: Previous hunk
- `n`: Next file
- `p`: Previous file
- `g`: Go to specific line

### Review Actions
- `a`: Accept current hunk
- `r`: Reject current hunk (with optional comment)
- `e`: Edit comment on rejected hunk
- `!`: Force accept all remaining hunks

### View Controls
- `d`: Toggle diff view (split/unified)
- `/`: Search hunks
- `?`: Show hunk stats
- `h`: Show this help

### File Operations
- `c`: Show context lines
- `w`: Wrap long lines
- `m`: Toggle mouse mode

### Control
- `u`: Undo last action
- `q`: Quit (prompt if changes)
- `Ctrl+C`: Force quit

## Understanding Hunks

Each hunk is a contiguous block of changes in a file:
- **Green** lines: Added
- **Red** lines: Removed
- **Cyan** lines: Context (unchanged)

## Comments on Rejections

When you reject a hunk, you can add a comment explaining why. Examples:

```
- Syntax error: should use let not const
- Logic issue: off by one error
- Style: doesn't match project conventions
```

Comments are included in Claude's feedback.

## Exit Codes

- `0`: All hunks accepted
- `1`: Error occurred
- `2`: Some hunks rejected (feedback sent to Claude in hook-mode)

## Direct CLI Usage

```bash
# Review staged changes
diffy --staged

# Review against main
diffy --main

# Review against specific ref
diffy --ref origin/develop

# Pipe from git
git diff main | diffy | git apply
```

## Troubleshooting

**Can't navigate?**
- Ensure terminal supports 256 colors
- Try: `echo $TERM`

**Mouse not working?**
- Enable in config: `mouse = true` in `~/.config/diffy/config.toml`
- Or in TUI: press `m` to toggle

**Comments not saving?**
- Try again with `e` key
- Ensure terminal isn't too narrow

For more info: `diffy --help`

See setup guide: `/diffy:setup`
