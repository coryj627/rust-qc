# Native fixed-control validation before treatment

The `just/` refresh below used the original author's oracle. That author was
subsequently invalidated because another source-bundled skill was readable.
These deterministic operator logs remain historical evidence, but do not define
the valid coding case. The replacement `author-just-v2` independently validated
both upstream regressions and nine neighbors using the native offline builds;
see `../cases/just/logs/validation-summary.md`.

The independent authors initially had only baseline dependency caches. Their
blocked native checks and supplemental source checks remain in the case bundles.
The operator then copied each pinned fixed snapshot into a separate control
workspace, fetched its Cargo dependencies through the original `sb fetch`, and
replayed the unchanged author-owned scoring runners through `sb run`. No toolchain
was installed, no production source or oracle assertion was changed, and no
treatment template/cache was changed. These are automated control checks, not
model treatments or rubric authorship.

| Case | Native oracle result | Additional evidence and limits |
| --- | --- | --- |
| heapless | Upstream oracle and both neighboring tests pass. | The complete author runner returns 0, including the existing suite and no-default-features check. |
| wikilink | Upstream oracle (1 test) and neighbors (4 tests) pass in debug and release. | Both full suites pass. The separate no-default-features check fails because this newer fixed release requires `hashbrown` when `std` is disabled; the baseline predates that feature contract. That failure remains recorded. |
| just | Both unchanged upstream regressions pass on the targeted recheck. | The first oracle invocation failed during dependency compilation with E0463 (`getrandom` unavailable to `uuid`). The neighboring and existing checks subsequently compiled and passed. The exact oracle was rerun without source/assertion changes and passed 2/2. The initial compiler failure is retained; its underlying artifact failure was not diagnosed. |

Each folder preserves fetch output, the resulting lockfile, the original native
runner status, and its detailed logs. The `just` folder also preserves the
targeted recheck command and output. No treatment result was available when
these controls were checked. Case rubrics and hidden assertions remain exactly
as authored.
