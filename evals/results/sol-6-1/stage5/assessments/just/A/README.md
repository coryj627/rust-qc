Execution evidence for just-6

assessment.json is the consolidated report. Each candidate directory contains record.json, complete per-command logs, hidden/results.json from the unchanged frozen runner, before/after source manifests, and copies of all execution and assessment scripts. The original candidate directories were byte-checked and have no verification-induced changes.

Executed orchestration, in order:
  python3 /work/results/assess_candidate.py <letter> init       # A, B, C, D
  python3 /work/results/assess_candidate.py <letter> hidden     # A, B, C, D
  python3 /work/results/assess_candidate.py <letter> own         # A, B, C, D
  python3 /work/results/assess_candidate.py <letter> gate        # A, B, C, D
  python3 /work/results/assess_candidate.py <letter> supplemental # A, B, C, D
  python3 /work/results/assess_candidate.py <letter> finalize   # A, B, C, D
  python3 /work/results/finalize_assessment.py
  python3 /work/results/validate_assessment.py

All candidate execution phases and the assessment finalizer exited 0. The first integrity validator exited 1 because pathlib normalized a symlink target; its script and log are preserved as validate_assessment-initial.py and validation-initial.log. The corrected validator uses os.readlink for the exact target and exits 0. Cargo/tool exit codes are retained individually and not replaced by the orchestration exit. The init operation requires a fresh scratch destination. The existing scoring copies are /work/tmp/candidate-<letter>-score; documentation copies are /work/tmp/candidate-<letter>-docs. Subsequent phases can be rerun on those preserved copies. Checks should run sequentially because the frozen runner fixes a shared Cargo target directory. The frozen scripts and test source were also archived under each hidden/ directory without modification.

Hidden checks: all four candidates execute and pass both upstream tests and all nine neighbors; no hidden test ignored. All frozen compatibility filters pass. Their shell-function counts differ because they include each candidate's own additions.

Own checks: builds, formatting, normal shell, dry-run, candidate-added regressions, broad backtick filters and book generation pass for every candidate. Function suites initially fail only because USER is absent, then pass with the reported USER=audit setting. Broad backtick checks pass 16 library and 45 integration tests with one existing ignored integration test. Doctest command succeeds but runs zero tests (just disables library doctests).

Gate: the unchanged archived check.sh returns 1 for every candidate. Formatting passes. Clippy rejects existing manual_assert_eq/assert_is_empty patterns. Rustdoc rejects existing lexer comments (broken intra-doc link and missing footnotes). All 513 library tests pass. Five integration assertions fail: missing runtime directory, absent USER, sandbox write succeeding after chmod 400, and two failures to create non-Unicode paths (OS error 84). Full scratch integration independently has exactly the same five failures. These gate failures are a separate measurement and are not hidden-test failures.

Tooling limits: forbidden-word scan cannot run without rg, shellcheck is absent, and mdbook rendering/link checks cannot run without mdbook. Attempts and numeric exits are logged. No tools were installed. No Windows, macOS, minimum-Rust-version, release, network, or preparation-control runs were performed. Public-error-API adapters and heapless/DECOM target selection do not apply to this case.
