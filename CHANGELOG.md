# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-02-12

### Added

- Configuration file support (`~/.config/diffy/config.toml`) with XDG base directory
- Configurable defaults: syntax highlighting, mouse, view mode, file tree
- Enhanced hook feedback with ```diff code blocks and 10KB truncation

### Changed

- Replaced `libc::isatty` with `std::io::IsTerminal` (removed libc dependency)
- Fixed stop.sh hook to use unstaged changes (removed incorrect `--staged` flag)

### Removed

- `libc` dependency (replaced with std library)

## [0.2.0] - 2025-02-12

### Added

- Interactive TUI with vim-style navigation (h/j/k/l, g/G)
- File tree sidebar with per-file change statistics (f key toggle)
- Side-by-side diff view mode (d key toggle)
- Keyword-based syntax highlighting for 9+ languages (h key toggle)
- Inline comments on hunks (c key)
- Text search within diffs (/ key, n/N navigation)
- Stats overlay with review progress (s key)
- Mouse support for scrolling and clicking (m key toggle)
- Undo support for review decisions (u key)
- Claude Code hook integration (--hook-mode)
- CLI mode with git integration (--staged, --head, --ref)
- JSON output mode (--json)
- Auto-apply with revert support (--apply, --restore)
- Backup rotation (up to 5 backups)
- Terminal cleanup guard for panic-safe shutdown

### Changed

- Reduced visibility scope with pub(super) for internal types
- Removed dead_code allows in favor of proper API exposure

## [0.1.0] - 2025-01-15

### Added

- Core unified diff parser
- Basic TUI diff viewer with accept/reject per hunk
- Pipe mode: `git diff | diffy | git apply`
- Diff output writer

[0.3.0]: https://github.com/jaykang-heo/diffy/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/jaykang-heo/diffy/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jaykang-heo/diffy/releases/tag/v0.1.0
