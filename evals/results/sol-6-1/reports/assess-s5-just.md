| Candidate | Hidden tests | Function tests with `USER=audit` | Gate exit | Evidence |
|---|---:|---:|---:|---|
| A | 2/2 upstream, 9/9 neighbors | 93 passed | 1 | [Record](/work/results/A/record.json) |
| B | 2/2 upstream, 9/9 neighbors | 93 passed | 1 | [Record](/work/results/B/record.json) |
| C | 2/2 upstream, 9/9 neighbors | 91 passed | 1 | [Record](/work/results/C/record.json) |
| D | 2/2 upstream, 9/9 neighbors | 92 passed | 1 | [Record](/work/results/D/record.json) |

All authored regressions, builds, formatting, compatibility checks and book generation passed. Each gate separately encountered existing Clippy/rustdoc diagnostics and five environment-related integration failures. Missing `rg`, `shellcheck` and `mdbook` blocked their respective checks.

Submission files remained unchanged. Commands, numeric exits, complete logs and limitations are recorded in [assessment.json](/work/results/assessment.json).