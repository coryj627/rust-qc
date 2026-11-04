# Changelog

Each version lists what changed in the rules and scripts. A rule change names the evaluation result that prompted it. The version in `metadata.version` in `SKILL.md` matches the latest released entry here, and each release has a matching tag, such as `v1.0.0`.

## Unreleased

### Fixed

These came from a review of the repository, not from an evaluation.

- `check.sh` now stops with exit 2 on rustup older than 1.28. Those versions have no way to turn off automatic downloads, so the statement in 1.0.0 that the script never installs a toolchain did not hold for them.
- The README no longer says 1.0.0 is the final evaluated version plus later additions. The evaluated versions are not in the repository, so that could not be confirmed. It now says only what the repository's history shows.

### Documentation

- The README now reports the answer-key evaluations of 1.0.0 on Opus 5.5 and Sol 6.1 and the Sol 6.1 coding runs. They did not show the skill helping either model find more known bugs. No rule or script changed because of them.

## 1.0.0 - 2026-11-02

The first versioned release. It summarizes the changes made so far. The earlier development versions were not numbered, so they cannot be pinned or compared directly.

### Rule changes

The first version tied the baseline on a parser written from a spec and on a planted bug, and it caused three defects of its own. These rules were rewritten because of them, and re-tests on two models came out clean.

- **Library errors let callers branch without parsing text.** The first version produced an error type callers could not branch on.
- **Every new dependency is a cost, dev-dependencies included.** The first version added an unneeded test dependency. Adding `proptest`, about 50 crates in `Cargo.lock`, to a 100-line parser was judged a cost with no gain over an exhaustive loop.
- **Structure matches size.** The first version created needless modules.

One rule came from the two real fixes, where the skill scored 11.5 against 11 of 12, and 8.5 against 8 of 9:

- **Human text gets Unicode-aware handling.** On the first fix, the grader still preferred the baseline's Unicode-correct text search.

### Audit results at this point

These results did not each prompt a rule change. They are the baseline to compare later versions against.

- On a crate with 18 planted defects, and on three real RustSec advisories checked out before their fixes, the skill matched the baseline. Both found every serious defect and all three advisories.
- On alacritty_terminal 0.25.0, scored against eight bugs fixed upstream after that release, two runs without the skill found 5 of 8 each. Two runs with the final development version found 5 and 4.
- The number of answer-key bugs found was stable between identical runs. Overall coverage swung by up to 8 Major-or-worse findings, so one run says little about coverage.
- On a medium-sized PDF accessibility library, an earlier version found two small files that crash the whole application, which no other run found.

Each version ran once or twice per case, the graders are the same model family as the agents they graded, and no person who writes Rust has reviewed the rules yet.

### Added

- A warning that auditing a crate runs its code, in the README and in Step 1 of `SKILL.md`.
- `license`, `compatibility` and `metadata.version` in the `SKILL.md` frontmatter.
- CI for the scripts, tests and spec validation.
- Tests for `check.sh`.

### Fixed

These came from a review of the repository, not from an evaluation.

- `check.sh` aborted on bash 3.2, the macOS default.
- `check.sh` could install nightly or a project's pinned toolchain through rustup. It now never installs a toolchain, and a missing pinned toolchain stops it with exit 2.
- `check.sh` missed nightly-only rustfmt options when stable rustfmt also found a diff.
- `check.sh` exited 1 instead of 2 when the crate root was missing.
- `audit_scan.py` skipped ordinary source modules named `data` or `build` inside `src/`.
- The scan tests passed if the scanner crashed on the `clean` or `binonly` fixture.
