| Candidate | Hidden checks | Debug/release suites | Package rustdoc | Gate exit | Evidence |
|---|---:|---|---|---:|---|
| A | 10/10 passed | Passed | Passed | 1 | [Logs](/work/results/A/README.md) |
| B | 10/10 passed | Passed | Passed | 1 | [Logs](/work/results/B/README.md) |
| C | 10/10 passed | Passed | Passed | 1 | [Logs](/work/results/C/README.md) |
| D | 10/10 passed | Passed | Passed | 1 | [Logs](/work/results/D/README.md) |

All passed formatting, no-default-features checks/tests, and serde/SIMD tests. A/B’s focused Miri checks passed.

Gate failures are separate: Clippy lint errors and workspace documentation/tests blocked by missing `llvm-objdump`. Reported generated-input probes for A–C remain unrun because reproducers were absent.

Submissions remained unchanged. Commands, exits, complete logs, adaptations, and limitations are recorded in [assessment.json](/work/results/assessment.json).