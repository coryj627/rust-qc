| Candidate evidence | Hidden checks | Default suite |
|---|---:|---:|
| [A](/work/results/A/README.md) | 46 passed, 0 failed | 177 passed |
| [B](/work/results/B/README.md) | 46 passed, 0 failed | 180 passed |
| [C](/work/results/C/README.md) | 46 passed, 0 failed | 180 passed |
| [D](/work/results/D/README.md) | 46 passed, 0 failed | 178 passed |

B and C also passed their no-default-feature suites and seven Miri regressions each.

Every standalone terminal gate exited **1**: strict Clippy and rustdoc failed, tests passed, and formatting was skipped. These are recorded separately from hidden results.

Submission hashes remain unchanged. Integrity verification was blocked by the missing `prepared.sha256` manifest.

Commands, numeric exits, complete logs, adaptations, and reproduction scripts are indexed in [assessment.json](/work/results/assessment.json).