Created:

- [case.md](/work/prepared/case.md): verbatim task and offline build instructions, without hidden-test contents or fix hints.
- [rubric.md](/work/prepared/rubric.md): 12-point rubric for two independent blind graders, plus separate harm scores.
- [hidden/](/work/prepared/hidden/README.md): verbatim upstream regressions, nine justified neighboring cases, provenance, and test integration scripts.
- [scoring.md](/work/prepared/scoring.md): exact candidate-scoring commands.
- [Validation logs](/work/prepared/logs/validation-summary.md) and integrity records.

Executed offline compilation and selected tests against pristine source copies:

| Check | Baseline | Fixed |
|---|---|---|
| Upstream regressions | Both fail as intended | Both pass |
| Nine neighbors | 3 pass, 6 fail | All pass |
| Existing shell/dry-run/backtick checks | Pass | Pass |

Both supplied snapshots remain unchanged. No candidate fix was implemented, and no tools or dependencies were installed.

Limitations: validation used Linux/Bash and the installed toolchain; Windows, minimum supported Rust, and the full suite were not tested. Git history was unavailable to authenticate the supplied commit. Private materials must be excluded from the treatment workspace as described in scoring.md.