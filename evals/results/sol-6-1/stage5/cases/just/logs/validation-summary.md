# Validation command summary

All commands were run in isolated copies, offline and locked, using the installed toolchain. No treatment run or candidate fix was performed.

| Source | Check | Exit | Result |
|---|---|---:|---|
| baseline | [compile](baseline/compile.log) | 0 | Integration compilation passed |
| baseline | [upstream](baseline/upstream.log) | 101 | FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1262 filtered out |
| baseline | [neighbors](baseline/neighbors.log) | 101 | FAILED. 3 passed; 6 failed; 0 ignored; 0 measured; 1255 filtered out |
| baseline | [existing-shell](baseline/existing-shell.log) | 0 | ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1258 filtered out |
| baseline | [existing-dry-run](baseline/existing-dry-run.log) | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1263 filtered out |
| baseline | [existing-backticks](baseline/existing-backticks.log) | 0 | ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1262 filtered out |
| fixed | [compile](fixed/compile.log) | 0 | Integration compilation passed |
| fixed | [upstream](fixed/upstream.log) | 0 | ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1790 filtered out |
| fixed | [neighbors](fixed/neighbors.log) | 0 | ok. 9 passed; 0 failed; 0 ignored; 0 measured; 1783 filtered out |
| fixed | [existing-shell](fixed/existing-shell.log) | 0 | ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1786 filtered out |
| fixed | [existing-dry-run](fixed/existing-dry-run.log) | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1791 filtered out |
| fixed | [existing-backticks](fixed/existing-backticks.log) | 0 | ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1790 filtered out |

Baseline primary failures are intended behavior failures: shell commands run and exit 1. Baseline has six dry-run neighbor failures and three compatibility neighbor passes. Fixed passes both primary tests and all nine neighbors. Existing selected checks pass on both snapshots. PowerShell checks return early on Linux.

No missing fixed-control dependencies. Full integration suite, Windows behavior, and minimum supported Rust version were not validated. Fixed emits its pre-existing deprecated Clippy lint-name warning.

Integrity audit: both original source snapshots unchanged; in each control, only tests/lib.rs plus two added hidden test modules differ. Upstream test blocks are verbatim. Symlinks preserved. See integrity.json and source-manifests.json.
