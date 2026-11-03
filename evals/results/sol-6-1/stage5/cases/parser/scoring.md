# Sandbox scoring commands

The treatment receives only `/work/prepared/case.md`, an exact copy of
`/work/spec.md`. Keep `/work/prepared/hidden/`, the rubric/scoring files, all
preparation logs, `/work/reference/` and negative controls outside the treatment
case/environment. Run grading after the treatment has produced `/work/crate`.
The grading sandbox must have that crate and the prepared grading artifacts;
the reference tree is not required and should not be provided to blind graders.

No treatment was run during preparation. Do not use preparation validation logs
as evidence about a submission. No automatic numeric score is computed: two
blind graders independently apply `rubric.md` to actual artifacts and check logs.

## Exact treatment check command

Run this command with `sandbox_exec` (timeout 240 seconds; if killed, keep already
written logs and classify incomplete checks under the rubric):

```sh
/work/prepared/hidden/run_checks.sh /work/crate /work/tmp/small-config-scoring
```

The script disables toolchain auto-installation and Cargo network access, keeps
all build/executable artifacts under `/work/tmp`, and runs these exact commands
with `RUSTUP_AUTO_INSTALL=0`, `CARGO_NET_OFFLINE=true`, and
`CARGO_TARGET_DIR=/work/tmp/small-config-submission-target`:

```sh
cargo check --manifest-path /work/crate/Cargo.toml --offline --lib
cargo test --manifest-path /work/crate/Cargo.toml --offline --all-targets
cargo test --manifest-path /work/crate/Cargo.toml --offline --doc
/work/prepared/hidden/run_api_tests.sh /work/crate
cargo fmt --manifest-path /work/crate/Cargo.toml --check
cargo clippy --manifest-path /work/crate/Cargo.toml --offline --all-targets
```

It captures each command's output and exit status separately and continues after
failure. Its own exit status is not an overall pass/fail. Read all results with:

```sh
cat /work/tmp/small-config-scoring/status.tsv
cat /work/tmp/small-config-scoring/library.log
cat /work/tmp/small-config-scoring/submitted-tests.log
cat /work/tmp/small-config-scoring/doctests.log
cat /work/tmp/small-config-scoring/public-api.log
cat /work/tmp/small-config-scoring/fmt.log
cat /work/tmp/small-config-scoring/clippy.log
```

Status 0 means that individual command completed successfully. A nonzero status
requires its log to distinguish a product failure from a blocked check. Inspect
source/docs/tests too; all-green checks alone cannot award all 12 points.
Rustfmt and clippy are diagnostics, not mandatory style or warning-free gates.
Review actual correctness warnings individually. Do not install tools, fetch
packages, rewrite the submission, or introduce stricter lint settings.

The API runner creates an isolated consumer crate under `/work/tmp`, depends on
`small_config` by path, and copies in `hidden/public_api.rs`. It does not insert
hidden test files in the submission or assume a `src/lib.rs` implementation path.
It overrides its own build target to `/work/tmp/small-config-api-target` and
cleans the temporary harness on exit. Cargo may create/update a submission
lockfile during normal verification; do not treat that grading side effect as
an authored unnecessary change. Runner exit 2 indicates a missing manifest or
runner invocation issue; Cargo test failures normally exit 101. Consult logs.

## Hidden behavior coverage and scoring interpretation

`public_api.rs` contains 24 external integration tests, using only `parse`,
`Entry`, and `ParseError`. They cover the specified syntax, valid and invalid
quotes/escapes, duplicate/order/case behavior, CRLF and final-line boundaries,
Unicode, ASCII-only layout, physical error lines, call independence and a bounded
no-panic corpus. Escape combinations have expected values specified directly by
the format, not computed by a second parser. No reference code is linked.
The error-trait check is compile-time. Diagnostic checks allow arbitrary wording
and look for the correct complete decimal line number, including multiple-digit
line numbers; manual review can accept an equally clear non-decimal/spelled-out
line-number representation if the automated heuristic misses it. They never
inspect variants, field layout, constructors, `source()`, or exact error messages.

Use rubric subcriteria, not `passed_tests / 24 * 12`. For example, a compilation
failure means tests were not executed; it does not establish that Unicode parsing
or duplicate detection failed. Partial behavior within a failed aggregate test
can be assessed from its assertion/source evidence. A test passes only for the
finite cases it exercises; it is not a proof for all strings.

## Preparation validation (author-only, not blind-grader evidence)

Preparation used Rust/Cargo 1.99.1 and the installed rustfmt/clippy. The independent
reference has no external dependencies. Reproduction, outside treatment/grading:

```sh
/work/reference/validate.sh
```

That script runs the same API runner on `/work/reference`, then rustfmt `--check`
and clippy `--all-targets -- -D warnings` on the reference. It also runs two
intentionally broken reference copies, asserts Cargo test failure rather than a
compiler/setup error, and verifies expected behavioral failures in their logs.
The strict reference lint check is preparation hygiene, not a treatment rule.

Preserved preparation logs:

- `/work/prepared/reference-api.log`: 24 passed, 0 failed.
- `/work/prepared/reference-fmt.log`: successful check (empty output).
- `/work/prepared/reference-clippy.log`: successful check.
- `/work/prepared/negative-unicode.log`: 22 passed, 2 failed; broken quoted
  Unicode copying is detected.
- `/work/prepared/negative-boundary.log`: 21 passed, 3 failed; omitted CRLF CR
  removal is detected.
- `/work/prepared/validation-summary.log`: concise validation outcomes.
- `/work/prepared/reference-runner-status.log`: all six scoring commands
  completed successfully in a smoke run on the reference (its own unit/doc test
  sets are empty; the external API suite is the substantive behavioral check).
- `/work/prepared/preparation-audit.log`: installed tool versions, case-copy
  identity, shell syntax and rubric point-accounting checks.

Reference code and both mutations live only under `/work/reference`; they are
not part of the treatment task. These checks validate suite execution and
sensitivity to two representative bugs, not every possible incorrect parser.
No treatment implementation has been evaluated or scored in preparation.
