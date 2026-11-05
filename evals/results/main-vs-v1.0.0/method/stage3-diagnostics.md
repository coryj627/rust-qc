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

## Brief test

Four new moka audits ran for the v1.0.0 arm with the earlier rounds' exact brief, from `evals/results/method/write_briefs.py` with the skill paragraph. The only change was "It is aarch64 Linux, so x86-only code cannot run natively." to "It is x86_64 Linux." Everything else was as in this round: `claude -p`, the same flags, model, effort, sandbox and template.

One agent graded the four redacted reports with the earlier rounds' grader brief, as in the regrading test. It found 0 of 20; the earlier rounds found 17 of 20. The audits used 4 to 10 clock minutes, wrote 509 to 1,136 words and cost 4.38 to 6.02 USD each, 20.58 USD in all, plus 0.12 USD for grading.

So the Report wording does not explain the drop. What remains is how agents are started (`claude -p` here, subagents of an interactive session before) and the effort level, which no event confirms.

## Subagent test

Two moka audits for the v1.0.0 arm ran as subagents of an interactive Claude Code session, as the earlier rounds ran them: `claude-opus-5-5` at high effort, the earlier rounds' brief (with the x86_64 line), the same workspaces, template and sandbox. They used 12 and 14 clock minutes, 90 and 137 tool calls, and about 287,000 and 309,000 tokens. Neither read outside its workspace, used the network or built on the host, though both ran ordinary host commands such as `mkdir`, `cp` and `awk`, which a subagent is not prevented from running.

One agent graded both redacted reports with the earlier rounds' grader brief: 0 of 10. Both reports found the same bugs that are not in the key as this round's `claude -p` audits did, for example `invalidate_entries_if` skipping one entry per batch and `remove` returning expired values.

So neither the Report wording nor the way agents are started explains the drop. Every setup tried here stops at 4 to 33 clock minutes and finds few or none of moka's key bugs, where the earlier rounds' runs used 15 to 36 minutes and found 17 of 20.
