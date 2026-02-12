---
name: review-diff
description: Review a diff of code changes interactively with diffy. Use when Claude Code generates a set of code changes and you want to accept/reject them hunk-by-hunk before applying.
---

# Diff Review Skill

## When to Use

Use this skill when:
- Claude has just generated or modified code
- You want to review changes before applying them
- You need to accept some changes and reject others
- You want structured feedback sent back to Claude

## How It Works

1. **Generate or fetch the diff**
   - Claude creates code changes
   - Stop hook detects changes

2. **Launch interactive review**
   - Runs: `diffy --hook-mode --apply`
   - Opens TUI with diff hunks

3. **Review each hunk**
   - Press `a` to accept
   - Press `r` to reject (with optional comment)

4. **Get feedback**
   - Rejected hunks auto-revert
   - Feedback shown to Claude
   - Claude refines the code

## Example Workflow

```
Claude: "I'll add error handling to the login function"
[Code generated]
[You review the changes in diffy TUI]
You: "Accept this error handling pattern"
[Some hunks have issues]
You: "Reject - needs async/await syntax"
[Feedback sent to Claude]
Claude: "I see, let me fix the async syntax"
[Code updated and regenerated]
```

## Keyboard Commands

- `a`: Accept hunk
- `r`: Reject with comment
- `e`: Edit comment
- `n`/`p`: Next/previous file
- `j`/`k`: Next/previous hunk
- `q`: Quit

See `/diffy:help` for full shortcuts.

## Integration with Claude Code

The Stop hook automatically runs diffy after:
- You ask Claude to generate/modify code
- Claude Code writes/modifies files
- The review window opens immediately

You can review and provide feedback within the Claude Code workflow seamlessly.
