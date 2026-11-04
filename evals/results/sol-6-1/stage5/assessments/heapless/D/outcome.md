Candidate D: all frozen checks passed (scorer exit 0).

- Hidden: upstream 1/1 and neighbors 2/2; zero ignored.
- Default: 187 unit + 9 integration + 177 doctests passed.
- Focused IndexMap and Miri: 23/23 each.
- Compatible features with warnings denied: 221 unit + 9 integration + 184 doctests passed.
- No-default-features library check and strict rustdoc passed.
- Stable formatting passed with nightly-option warnings. Installed nightly lacks rustfmt, so nightly formatting was blocked (exit 1).
- Archived gate exit 1: Clippy failed; rustdoc and tests passed; formatting skipped. Direct Clippy exit 101 reproduces the unchanged baseline diagnostic-message multiset.
- Candidate regression transferred alone to unchanged baseline compiled and failed the retained lookup assertion (exit 101, expected regression evidence).
- Submission files unchanged, including Cargo.lock. Only grading-created hidden tests were added to scoring scratch; no generated lockfiles attributed to candidate.

Commands and exits: checks.json and /work/results/assessment.json. Full logs: this directory and frozen-logs/. Source and suppression review: source-review.json. Integrity: integrity.json. Reproduction scripts are retained beside the logs. No quality score assigned.
