# Correctness-focused review prompts, version 2

These prompts were requested after the first Sol evaluation. They make the ordinary Rust code-review task explicit and exclude dedicated security testing. They are now used in a [separately labeled v2 run](../../../review-v2/README.md); the original evaluation's scores do not use them.

The existing main prompt asks to report "defects"; the short prompt asks for "problems." Neither spells out the review scope. These replacements ask for documented behavior, API/error handling, routine regression coverage, build/documentation results and maintainability, with evidence of the consequence for ordinary callers.

Security testing is not required or requested. The prompts exclude a dedicated security assessment, threat modeling, exploitability analysis and exploit development. Ordinary correctness findings may still include panics or memory-safety errors encountered during normal review; the reviewer is not asked to investigate possible security impact. An inquiry that requires dedicated security testing is recorded as outside scope. This narrows the requested work; it does not declare the original corpus incapable of having security-relevant defects.

## Templates

- [main.md](main.md): the detailed 45-minute review.
- [short.md](short.md): a concise 15-minute review.
- [with-skill.md](with-skill.md): the main prompt's treatment-only skill paragraph.

Render the templates with Python `str.format` using the pinned project values from `evals/results/method/write_briefs.py`:

- `name`: the project name and release.
- `path`: the review source path, relative to `/work`, such as `crate/src/`.
- `build_notes`: the project's existing build/test instructions.
- `skill_instructions`: the contents of `with-skill.md` for the main treatment arm, or an empty string for its baseline. Only the main template has this field.

The short treatment arm continues to receive the sole skill through the available-skills catalog, as in the original short cell. Its task text is identical in both arms. Baselines receive no skill or skill catalog entry. The source-tree scan must still reject any other skill in an input workspace.

## Evaluation boundary

The original `prepare_audits.py`, `prepare_stage4.py`, frozen schedules, reports, scores and skill remain the record of the first evaluation. These templates are not wired into those historical generators and do not retry a blocked run. A future run requires a new declared condition and fresh workspaces; record this prompt version and the rendered prompt hashes before dispatch.

The more specific wording changes the task, especially the originally one-line short cell. Results would not be directly interchangeable with the original audit comparison. Before a new scored study, review key eligibility against the stated scope and freeze that decision for both arms; do not score an explicitly out-of-scope inquiry as a miss or select exclusions after seeing new reports.

Prompt clarification does not establish the cause of the previous provider blocks or guarantee that they will stop. Preserve provider errors and follow the existing stop policy if a future permitted run is blocked. These prompts do not ask a model to ignore safeguards or conceal security work.
