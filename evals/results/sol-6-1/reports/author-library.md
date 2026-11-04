Created:

- [case.md](/work/prepared/case.md): exact, unchanged spec copy.
- [rubric.md](/work/prepared/rubric.md): 12-point rubric for two blind graders, separate harm scores, and blocked-check guidance.
- [hidden tests](/work/prepared/hidden/public_api.rs), error-category adapter support, and runner.
- [scoring.md](/work/prepared/scoring.md): exact sandbox commands.
- Independent reference and negative controls under `/work/reference`.
- [validation record](/work/prepared/validation.md) and logs.

Checks run offline: reference passed all 11 integration tests; public-only mode passed 8; the reference’s documentation example passed. Both deliberately broken controls failed as intended. The case/spec byte comparison passed.

Limitations: candidate error-category checks require a reviewed adapter. Invariant enforcement, documentation, dependencies, and scope also require manual review. No treatment run occurred; nothing was installed or fetched.