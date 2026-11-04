Created:

- [case.md](/work/prepared/case.md), byte-identical to the full spec.
- [rubric.md](/work/prepared/rubric.md): 12-point quality rubric, separate harm scoring, and instructions for two blind graders.
- [hidden/](/work/prepared/hidden): 24 public API integration tests and check runners.
- [scoring.md](/work/prepared/scoring.md): exact sandbox commands.
- Independent reference and two negative controls under `/work/reference`.

Validation passed: the reference passes all 24 tests; the Unicode control fails 2 tests and the CRLF control fails 3. Rustfmt, clippy, script syntax, case identity, rubric accounting, and all six scoring-runner commands passed. Logs are preserved under `/work/prepared/`.

No treatment was run. Validation covers finite cases and two representative bugs; it does not prove exhaustive correctness. Nothing was installed or fetched.