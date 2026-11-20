# Workspace boundary regression evidence

Source: the reported live-service workspace symlink replacement bug. No plan file
was supplied. Tests were written and executed before production changes.

## User journey

An inspection must remain within the workspace bound at service startup. If the
workspace resolves elsewhere on a later run, that run must fail without publishing
an inventory, release its worker slot, and permit an explicit retry after repair.

## RED and GREEN

All Cargo commands below were run from the repository root through
`MISE_AUTO_INSTALL=0 /Users/patricklarocque/.local/bin/mise exec --`.

Command: `cargo test --locked --manifest-path backend/Cargo.toml --test workflows`

- RED, before production edits: `13 passed; 3 failed`. Both scanner regressions
  returned a report containing `outside-only.txt`; the run regression returned
  success after replacement of the bound workspace.
- GREEN, after the fix: `16 passed; 0 failed` using the same command.

Implementation: validate the stored canonical path at the start of each run,
before and after traversal, and before publishing. Reject root symlinks and
changed canonical resolution, including changes through ancestors. Configure
WalkDir with `follow_root_links(false)` as well as `follow_links(false)` and
reject a traversal root that is no longer a directory.

## Test guarantees

All tests below are in [`backend/tests/workflows.rs`](../../backend/tests/workflows.rs) and ran on macOS. The three
new symlink tests are gated with `cfg(unix)`.

| Test | Guarantee | Result |
| --- | --- | --- |
| `scanner_rejects_workspace_replaced_with_symlink` | A bound root replaced by a symlink produces an inspection error. | PASS |
| `scanner_rejects_workspace_with_replaced_ancestor` | Replacing an ancestor with a symlink fails even when the final root is a directory. | PASS |
| `each_run_revalidates_workspace_and_persists_boundary_failure` | A later run rejects replacement, fails at Planner, persists failed status with no report, frees capacity, and succeeds on explicit retry after restoring scope. | PASS |
| Existing successful inspection, alias binding, exclusions and recovery tests | Normal inspections and startup alias canonicalization continue to work. | PASS |

## Validation and limits

- `cargo test --locked --manifest-path backend/Cargo.toml`: 20 tests passed
  (3 API, 1 persistence, 16 workflow tests).
- `cargo clippy --locked --manifest-path backend/Cargo.toml --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.
- Coverage percentage was not measured: `cargo --list` contains no coverage
  subcommand. The skill's 80% coverage target is unverified; no tooling was installed.
- Path revalidation does not provide atomic filesystem isolation. Concurrent
  swaps between validation and filesystem operations, especially swaps restored
  before the next validation, require descriptor-relative traversal for stronger
  guarantees. The tests deterministically cover replacement before a run; they
  do not simulate swaps during traversal.
- Separate RED/GREEN checkpoint commits were not created during implementation;
  commits had not been authorized at that point. The later publication request
  packages the fix, regression tests, and related documentation together, with
  the original RED/GREEN evidence preserved here. No separate refactor was performed.
