Created [case.md](/work/prepared/case.md), [rubric.md](/work/prepared/rubric.md), hidden fixtures, [scoring.md](/work/prepared/scoring.md), and [validation report](/work/prepared/validation.md) with command logs.

Checks and outcomes:

- Baseline oracle fails as intended in debug and release.
- Baseline existing tests: 1,250 tests and 5 doctests pass per profile; no-default-features check passes.
- Unchanged fixed source passes the oracle and four neighboring tests in supplemental debug and optimized builds.
- Source hashes confirm both snapshots remain unchanged. No candidate fix was implemented.

Limitation: native fixed Cargo validation is blocked by missing offline dependency `dlmalloc`. Supplemental builds bypass workspace resolution and release LTO; they are not claimed as native Cargo passes.