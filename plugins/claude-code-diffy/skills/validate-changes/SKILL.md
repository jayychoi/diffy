---
name: validate-changes
description: Validate that applied changes are syntactically correct and don't break existing functionality. Use after code generation or refactoring.
---

# Change Validation Skill

## When to Use

Use this skill when:
- Code changes have been applied
- You want to ensure they compile/run correctly
- You need quick validation before committing
- Testing after code generation

## Validation Steps

### 1. Format Check
```bash
cargo fmt --check
```
Ensures code follows Rust formatting standards.

### 2. Lint Check (Clippy)
```bash
cargo clippy -- -D warnings
```
Catches common mistakes and anti-patterns.

### 3. Compile
```bash
cargo build
```
Verifies the code compiles without errors.

### 4. Run Tests
```bash
cargo test
```
Ensures existing functionality still works.

## Quick Validation

For a fast check without tests:
```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo build
```

## Full Validation (Recommended)

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo build && cargo test
```

## Understanding Output

**Format Issues:**
```
error: `should be` ... (run `cargo fmt` to fix)
```
→ Run `cargo fmt` to auto-fix

**Clippy Warnings:**
```
warning: field is never read: `field`
```
→ Remove unused code or address the warning

**Compile Errors:**
```
error[E0425]: cannot find value `x` in this scope
```
→ Check variable names and scope

**Test Failures:**
```
thread 'test_name' panicked at 'assertion failed'
```
→ Review the test and the code change

## After Validation

If all checks pass ✓:
- Changes are ready
- Safe to commit/push
- Proceed with workflow

If issues found ✗:
- Review the error messages
- Ask Claude to fix specific issues
- Run validation again

## Example Session

```
You: "Generate a function to parse JSON"
Claude: [Writes parse function]
[Stop hook runs diffy review]
You: Accept all changes ✓
[Validation runs]
```bash
cargo test  # All tests pass
```
→ Ready to commit!
```

## Troubleshooting

**Tests are timing out?**
- Run single test: `cargo test test_name -- --nocapture`
- Check for infinite loops

**Build fails after changes?**
- Check compiler error message
- Review what Claude changed
- Ask Claude to fix compilation error

**Tests fail but code looks right?**
- Run tests with output: `cargo test -- --nocapture`
- Review test expectations
- Update tests if behavior changed intentionally

See `/diffy:help` for review commands.
