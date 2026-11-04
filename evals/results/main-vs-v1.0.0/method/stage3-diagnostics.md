# Stage 3 diagnostics

## Fewer known bugs found than in earlier rounds

After 5 projects, both arms find far fewer key bugs than the earlier Opus 5.5 rounds did on the same projects, scored against the same `match_rule` (`original`):

| Project | Earlier rounds, found | This round, found |
|---|---|---|
| moka | 17 of 20 | 3 of 20 |
| just | 31 of 44 | 8 of 44 |
| mlua | 33 of 36 | 7 of 28 |
| quinn-proto | 23 of 32 | 8 of 24 |
| pulldown-cmark | 24 of 40 | 2 of 40 |

Earlier rounds counted every key entry with a reproducer, so their totals are larger for mlua and quinn-proto.

## Regrading test

moka's four redacted reports were regraded by one agent with the earlier rounds' exact grader brief (`evals/results/method/key-grader-brief.md`, `match_rule` only, key entries with their original ids and fields). It found 4 of 20, against 3 of 20 from this round's graders. moka-2, moka-4 and moka-5 appear in none of the four reports, and their code is in the audited crate. So the drop comes from the audits, not from grading. Cost: 0.19 USD.

## What differs from earlier rounds

- Agents run as separate `claude -p` processes; earlier rounds ran them as subagents of an interactive session.
- The brief's Report section says "Use the skill's report format", where earlier briefs asked for each finding's location, what goes wrong, and how it was established.
- `--effort high` is accepted, but no event confirms the level the model ran at.
- This round's audits used 4 to 33 clock minutes; earlier rounds' used 15 to 36.

## zip

zip's four audits were stopped at 5 to 9 clock minutes when the driver was stopped, and wrote no report. They are not usable and need new runs.
