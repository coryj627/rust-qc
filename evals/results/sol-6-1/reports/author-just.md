Created in `/work/prepared/`:

- [case.md](/work/prepared/case.md): task wording and build instructions.
- [rubric.md](/work/prepared/rubric.md): 12-point rubric for two blind graders, with separate harm scores.
- `hidden/`: unchanged upstream regressions, eight neighboring cases, provenance, and test runners.
- [scoring.md](/work/prepared/scoring.md): exact commands and integration adaptations.
- [validation.md](/work/prepared/validation.md): outcomes, logs, checksums, and limitations.

Executed checks confirm both upstream tests fail on baseline for the intended bug. Supplemental tests produce four expected bug failures and four compatibility passes. Existing function, backtick, and dry-run checks pass with the documented environment setting.

**Fixed-control validation is blocked:** offline compilation cannot resolve `digest-io` 0.1.0. No fixed pass is claimed.

Both original source trees remain unchanged. No candidate fix or treatment run was performed. The operator must enforce hidden-file isolation before treatment.