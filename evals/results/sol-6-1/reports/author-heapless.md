Created:

- [case.md](/work/prepared/case.md): task wording and build instructions, without fix hints.
- [rubric.md](/work/prepared/rubric.md): 12-point rubric for two blind graders, with separate harm scores.
- [hidden/](/work/prepared/hidden): preserved upstream assertions, neighboring cases, and scoring scripts.
- [scoring.md](/work/prepared/scoring.md): provenance, adaptations, exact commands, and results.
- [logs/](/work/prepared/logs): command output and source-integrity records.

Validation confirmed the intended baseline lookup failure. Its existing default suite and library check pass. All three hidden tests pass against unchanged fixed source using the documented external manifest; 25 existing IndexMap tests also pass.

The fixed tree’s native build remains blocked by uncached `embedded-io`. No native fixed-build pass is claimed. Both original trees remain unchanged; no candidate fix or treatment run was performed.