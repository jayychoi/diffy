# Diffy Claude Code Plugin

Interactive TUI diff reviewer for Claude Code. Review code hunks one-by-one, accept/reject changes, and send structured feedback back to Claude.

## Quick Start

### Installation

1. Open Claude Code
2. Run `/plugin`
3. Add marketplace URL:
   ```
   https://github.com/jayychoi/diffy
   ```
4. Search for and install "diffy"

### Or via CLI

```bash
# Install diffy binary first
cargo install --git https://github.com/jayychoi/diffy diffy-tui

# Verify
diffy --version
```

## Features

- **Interactive TUI**: Review diffs hunk-by-hunk in terminal
- **Structured Feedback**: Automatic rejection + comments sent to Claude
- **Auto-integration**: Stop hook runs diffy after code changes
- **Git-native**: Works with `git diff | diffy | git apply` pipeline
- **Config File Support**: Customize via `~/.config/diffy/config.toml`
- **Keyboard-driven**: Efficient navigation and review workflow

## How It Works

```
[Claude generates code]
         ↓
[Stop hook triggers diffy]
         ↓
[You review hunks in TUI]
         ↓
[Accept/reject hunks]
         ↓
[Rejected hunks auto-revert + feedback sent]
         ↓
[Claude reads feedback and refines]
```

## Keyboard Shortcuts

### Navigation
- `j`/`k` or `↓`/`↑`: Move between hunks
- `n`/`p`: Next/previous file
- `g`: Go to line

### Review
- `a`: Accept current hunk
- `r`: Reject with comment
- `e`: Edit comment

### View
- `d`: Toggle diff view
- `/`: Search hunks
- `?`: Show stats
- `h`: Show help

### Control
- `u`: Undo
- `q`: Quit
- `!`: Force accept all

See `/diffy:help` in Claude Code for full reference.

## Usage

### With Claude Code (Automatic)

1. Ask Claude to generate/modify code
2. Stop hook auto-runs diffy
3. Review hunks in TUI
4. Rejected hunks auto-revert, feedback sent to Claude

### Direct CLI

```bash
# Review staged changes
diffy --staged

# Review against main
diffy --main

# Pipe from git
git diff | diffy | git apply
```

## Configuration

Create `~/.config/diffy/config.toml`:

```toml
[ui]
theme = "dark"           # light/dark
mouse = true             # enable mouse support

[feedback]
max_size = 10240         # bytes for hook feedback
```

## Commands

In Claude Code:

- `/diffy:setup`: Installation and configuration guide
- `/diffy:help`: Keyboard shortcuts and usage reference

## Troubleshooting

### Diffy not found?
```bash
which diffy
cargo install --git https://github.com/jayychoi/diffy diffy-tui
```

### Hook not running?
- Ensure plugin is enabled: `/plugin` → manage diffy → enable hooks
- Check git repository: `git status`
- Debug: `claude --debug` shows hook execution

### Changes not showing?
- Verify unstaged changes: `git status`
- Try: `git diff` to see what would be reviewed

## Architecture

- **Plugin Root**: `./plugin/`
- **Manifest**: `.claude-plugin/plugin.json`
- **Commands**: `commands/*.md` (auto-invoked by `/diffy:name`)
- **Skills**: `skills/*/SKILL.md` (Claude-triggered)
- **Hooks**: `hooks/hooks.json` (Stop hook config)
- **Scripts**: `hooks/*.sh` (Hook implementation)

## Support

- **Issues**: https://github.com/jayychoi/diffy/issues
- **Docs**: https://github.com/jayychoi/diffy#readme
- **Discussions**: https://github.com/jayychoi/diffy/discussions

## License

MIT
