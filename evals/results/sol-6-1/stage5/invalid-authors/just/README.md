Prepared case `just-6`:

- `case.md`: treatment task and offline workspace/build instructions.
- `rubric.md`: frozen 12-point rubric and separate harm dimensions for two independent blind graders.
- `hidden/`: unchanged upstream regressions, eight neighboring cases, provenance, staging and scoring helpers. Operator only.
- `scoring.md`: exact test application/run commands and integration adaptations. Operator only.
- `validation.md`: actual control outcomes and limitations.
- `logs/`: command outputs, exit codes, summaries, cache-gap inventory and integrity checks. `logs/initial/` retains superseded exploratory diagnostics.
- `source-sha256.json`: original source-file fingerprints.

Baseline fails both upstream regressions for the intended bug. Fixed runtime validation is blocked by missing offline dependencies, beginning with `digest-io` 0.1.0. No fixed pass is claimed. Enforce the holdback boundary described in `scoring.md` before any treatment.
