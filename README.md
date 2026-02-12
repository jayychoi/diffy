# diffy

Interactive TUI diff reviewer for Claude Code

[![CI](https://github.com/jaykang-heo/diffy/actions/workflows/ci.yml/badge.svg)](https://github.com/jaykang-heo/diffy/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/diffy-tui.svg)](https://crates.io/crates/diffy-tui)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A powerful terminal-based diff reviewer that lets you accept or reject individual hunks interactively. Built specifically to integrate with Claude Code workflows, but works standalone with any git repository.

## Features

- **Interactive TUI with vim-style navigation** — Navigate diffs with `j`/`k`, `g`/`G`, and all your favorite vim motions
- **Granular hunk control** — Accept or reject individual hunks, not just entire files
- **File tree sidebar** — See all changed files with per-file statistics (added/removed lines)
- **Side-by-side diff view** — Toggle between unified and side-by-side comparison modes
- **Syntax highlighting** — Keyword-based highlighting for Rust, TypeScript, JavaScript, Python, Go, Java, C/C++, and Ruby
- **Inline comments** — Add review comments to specific hunks for context
- **Text search** — Find specific changes across all diffs with `/` search
- **Stats overlay** — View review progress and navigate directly to files
- **Mouse support** — Optional mouse interaction for scrolling and selection
- **Undo review decisions** — Changed your mind? Press `u` to undo
- **Claude Code hook integration** — Automatically review Claude's changes and provide feedback
- **CLI mode with git integration** — Review staged (`--staged`), HEAD (`--head`), or any ref (`--ref`)
- **JSON output** — Programmatic access to review results with `--json`
- **Auto-apply with revert** — Use `--apply` to automatically revert rejected hunks
- **Pipe mode** — Classic Unix workflow: `git diff | diffy | git apply`

## Installation

### From crates.io (recommended)

Requires Rust 1.85.0 or later:

```bash
cargo install diffy-tui
```

The binary is installed as `diffy`.

### From GitHub Releases

Download the latest binary for your platform from the [releases page](https://github.com/jaykang-heo/diffy/releases).

### Verify installation

```bash
diffy --version
```

## Usage

### Basic usage

```bash
# Review unstaged changes in current directory
diffy

# Review staged changes
diffy --staged

# Review changes in HEAD commit
diffy --head

# Review changes against a specific ref
diffy --ref main
diffy --ref HEAD~3

# Review changes in a specific path
diffy -- src/
diffy --staged -- src/main.rs
```

### Pipe mode

Classic Unix workflow where diffy acts as a filter:

```bash
# Review and selectively apply changes
git diff | diffy | git apply

# Review staged changes and apply to working tree
git diff --staged | diffy | git apply
```

### Output formats

```bash
# Output as JSON (for scripting)
diffy --staged --json

# Standard unified diff output (default)
diffy --staged
```

### Auto-apply mode

Automatically revert rejected hunks after review:

```bash
# Review staged changes and auto-revert rejected hunks
diffy --staged --apply

# Restore the last backup if you made a mistake
diffy --restore
```

**Important**: `--apply` creates a backup at `.diffy/backup` before making changes. The backup rotates (keeps last 5 backups).

## Claude Code Integration

Diffy was built to integrate seamlessly with Claude Code's stop hooks. When Claude makes changes, diffy automatically reviews them and provides structured feedback.

### Setup

Add this to your `.claude/settings.json`:

```json
{
  "hooks": {
    "stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "diffy --staged --hook-mode --apply"
          }
        ]
      }
    ]
  }
}
```

### How it works

1. **Claude Code stops** after making changes (e.g., implementing a feature)
2. **Hook triggers** — The stop hook automatically runs `diffy --staged --hook-mode --apply`
3. **You review** — The TUI opens and you review each hunk (accept/reject)
4. **Rejected hunks are reverted** — Changes you reject are automatically removed
5. **Feedback to Claude** — Structured feedback is sent to Claude via stderr:
   ```
   [diffy review result]
   rejected 2 of 10 hunks.

   - src/main.rs (lines 45-52): rejected
     comment: this breaks error handling
   - src/lib.rs (lines 12-15): rejected

   please fix the rejected hunks and try again.
   ```
6. **Claude sees feedback** — Claude reads the rejection details and can adjust its implementation

### Exit codes

- `0` — All hunks accepted
- `2` — Some hunks rejected (in `--hook-mode`, triggers Claude Code feedback loop)
- `1` — Error occurred

### Why this matters

Without diffy, you'd need to manually revert Claude's unwanted changes or explain in prose what to fix. With diffy:

- **Visual review** — See exactly what changed before accepting
- **Instant feedback loop** — Claude knows precisely which hunks were rejected
- **Granular control** — Accept 90% of the work, reject the problematic 10%
- **Comments as context** — Your rejection comments guide Claude's next attempt

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Next hunk |
| `k` / `↑` | Previous hunk |
| `n` | Next file (or next search match if search is active) |
| `N` | Previous file (or previous search match) |
| `Ctrl+d` | Scroll down half page |
| `Ctrl+u` | Scroll up half page |
| `g` then `g` | First hunk (vim-style) |
| `G` | Last hunk |
| `Tab` | Jump to next pending (unreviewed) hunk |

### Review Actions

| Key | Action |
|-----|--------|
| `a` | Accept current hunk |
| `r` | Reject current hunk |
| `Space` / `Enter` | Toggle current hunk status |
| `A` | Accept all hunks |
| `R` | Reject all hunks |
| `u` | Undo last review decision |

### Comments

| Key | Action |
|-----|--------|
| `c` | Add/edit comment on current hunk |
| `Enter` | Submit comment (in comment mode) |
| `Esc` | Cancel comment editing |

### Views and Overlays

| Key | Action |
|-----|--------|
| `f` | Toggle file tree sidebar |
| `d` | Toggle side-by-side diff view |
| `h` | Toggle syntax highlighting |
| `s` | Toggle stats overlay |
| `?` | Show/hide help overlay |

### Search

| Key | Action |
|-----|--------|
| `/` | Enter search mode |
| `Enter` | Submit search query |
| `Esc` | Cancel search |
| `n` | Next match (when search is active) |
| `N` | Previous match |

### Other

| Key | Action |
|-----|--------|
| `m` | Toggle mouse support |
| `q` / `Esc` | Quit (with confirmation) |
| `y` / `Enter` | Confirm quit |
| `n` / `Esc` | Cancel quit |

## Configuration

Diffy currently has no configuration file. All behavior is controlled via command-line flags.

## Advanced Usage

### Combining with git

```bash
# Review and apply only the changes you accept from a stash
git stash show -p | diffy | git apply

# Review changes from a specific commit
git show abc123 | diffy | git apply

# Compare two branches interactively
git diff main..feature | diffy
```

### Using with --apply for automated workflows

```bash
# Review staged changes, auto-revert rejected ones
diffy --staged --apply

# If you made a mistake, restore the backup
diffy --restore

# The backup is stored at .diffy/backup/
ls -la .diffy/backup/
```

### JSON output for scripting

The `--json` flag outputs structured data:

```json
{
  "files": [
    {
      "old_path": "src/main.rs",
      "new_path": "src/main.rs",
      "raw_old_path": "a/src/main.rs",
      "raw_new_path": "b/src/main.rs",
      "is_binary": false,
      "hunks": [
        {
          "header": "@@ -10,5 +10,6 @@",
          "old_start": 10,
          "old_count": 5,
          "new_start": 10,
          "new_count": 6,
          "status": "accepted",
          "comment": null,
          "lines": [...]
        }
      ]
    }
  ]
}
```

## How It Works

1. **Parse diff** — Reads unified diff format from stdin or git
2. **Interactive review** — TUI lets you navigate and review each hunk
3. **Filter hunks** — Only accepted hunks are included in output
4. **Output** — Writes filtered diff to stdout (or JSON with `--json`)

The output is a valid unified diff that can be piped to `git apply`.

## Requirements

- **Rust 1.85.0+** (for building from source)
- **Git** (for CLI mode: `--staged`, `--head`, `--ref`)
- **Terminal** with color support (recommended)

## Troubleshooting

### "Not a git repository"

When using CLI mode (`--staged`, `--head`, `--ref`), diffy must be run inside a git repository. Use pipe mode if you want to review arbitrary diffs:

```bash
cat my.diff | diffy
```

### "No changes to review"

This means there are no changes in the requested scope:

- `diffy` — No unstaged changes
- `diffy --staged` — No staged changes
- `diffy --head` — HEAD commit has no changes

### Colors not showing

Ensure your terminal supports colors and has `TERM` set correctly:

```bash
echo $TERM  # Should be something like xterm-256color
```

### Mouse support not working

Press `m` to toggle mouse mode on. Some terminals may not support mouse events.

## Development

### Build from source

```bash
git clone https://github.com/jaykang-heo/diffy.git
cd diffy
cargo build --release
./target/release/diffy --version
```

### Run tests

```bash
cargo test
```

### Project structure

```
src/
├── main.rs          # Entry point, mode routing
├── cli.rs           # CLI argument parsing
├── parse.rs         # Unified diff parser
├── git.rs           # Git integration
├── hook.rs          # Claude Code hook mode
├── revert.rs        # Backup and revert logic
├── output.rs        # Diff and JSON output
├── model.rs         # Data structures
├── tty.rs           # TTY detection
└── tui/
    ├── mod.rs       # TUI main loop
    ├── state.rs     # Application state
    ├── input.rs     # Keyboard handling
    ├── render.rs    # UI rendering
    └── highlight.rs # Syntax highlighting
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Credits

Built with:

- [ratatui](https://github.com/ratatui-org/ratatui) — Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — Terminal manipulation
- [clap](https://github.com/clap-rs/clap) — CLI argument parsing

Created to enhance [Claude Code](https://claude.ai/claude-code) workflows.

## See Also

- [Claude Code documentation](https://docs.anthropic.com/claude/docs/claude-code)
- [git-add --patch](https://git-scm.com/docs/git-add#Documentation/git-add.txt--p) — Git's built-in interactive staging
- [tig](https://jonas.github.io/tig/) — Text-mode interface for git
