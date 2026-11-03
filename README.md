# Rust QC

Rust QC is an agent skill that keeps Rust code well organized, easy to understand, correctly documented and idiomatic. It covers three kinds of work: writing new code, changing existing code, and auditing code that already exists.

The skill lives in `.agents/skills/rust-qc/`. Its name, for agents that invoke skills by name, is `rust-qc`.

## What it does

**Writing new code.** The skill decides where the code belongs, models invariants with types, and shapes signatures and names to the Rust API Guidelines. It documents public items with runnable examples. Library errors let callers branch on a typed kind instead of parsing messages. Tests come first from checks that need no new dependencies.

**Changing existing code.** The skill reads the surrounding code before editing and matches the project's own conventions. A bug fix starts from a failing test that covers the whole class of bug, not only the reported input. Fixes address the root cause, keep public APIs and ABIs stable, and stay surgical. Formatting follows the project's rustfmt settings, including projects whose settings need nightly rustfmt.

**Auditing code.** A quick mechanical pass runs fmt, clippy, rustdoc, the tests, advisory lints and a heuristic scan. Most of the audit then goes to finding what the code gets wrong:

- realistic inputs run through the public API for each feature the code claims to handle
- a harness that feeds untrusted-input parsers short inputs, random mutations and crafted structures
- a read-through for soundness, error design and API shape, and for problems in code that talks to the operating system

The report is ordered by severity. Each finding is headed by a plain description of the problem, so it reads well with a screen reader and pastes cleanly into an issue.

**Auditing a repository runs its code.** `check.sh` and the audit's harnesses build and run the crate's build scripts, procedural macros and tests with your permissions. Audit untrusted code inside a sandbox, container or disposable VM. Only `audit_scan.py` is read-only: it reads the source without building or running it.

## Installing

Copy the `.agents/skills/rust-qc` folder into the `.agents/skills/` folder of your project, or into the user-level skills folder your agent reads. If your agent reads skills from a different folder, copy or link the `rust-qc` folder there.

Invoke it by name, for example "Use rust-qc to audit this crate." In testing, agents loaded it on their own for only about one Rust request in three.

## Requirements

- cargo, rustfmt and clippy
- python3, for the heuristic scan
- nightly rustfmt, only for projects whose `rustfmt.toml` uses nightly-only options
- Miri, recommended for crates that contain `unsafe`

The skill never installs toolchains or tools on its own.

## Layout

- `.agents/skills/rust-qc/SKILL.md`: the workflow for each mode, the rules that always apply, severity levels and the report format.
- `.agents/skills/rust-qc/references/`: the detailed rules, one file per area:
  - organization
  - API design
  - errors and panics
  - documentation
  - unsafe code
  - testing
  - tooling
  - how conflicts between sources are resolved
  - the audit checklist
  - notes on the exemplar crates
- `.agents/skills/rust-qc/scripts/check.sh`: the verification gate.
- `.agents/skills/rust-qc/scripts/audit_scan.py`: a scan for problems clippy cannot see.
- `tests/`: tests for both scripts.

## The scripts

`check.sh [--audit] [--target TRIPLE] [--features LIST] <crate-root>` runs `cargo fmt --check`, `cargo clippy -D warnings`, rustdoc with warnings denied, and `cargo test`.
- `--audit` adds an advisory clippy pass for documentation, unsafe and panic rules.
- `--target` lints and documents code behind a platform gate, such as Windows-only code checked from Linux.
- `--features` replaces `--all-features` for crates whose features cannot all be enabled together.

It exits 0 when every step passes, 1 when any step fails, and 2 when a precondition is missing. A toolchain the project pins but you have not installed counts as missing, because the script turns off rustup's automatic downloads.

`python3 -I audit_scan.py <crate-root>` reports glob re-exports, weasel-word type names, ASCII-art comments, design-history docs, parameter tables, bare TODOs, empty `expect` messages and print macros in library code. It skips benchmark, fuzz and data folders, and treats tests and examples as non-production code. Inside `src/` it skips only test data and treats only `tests` as non-production, because folder names there are module names. A workspace member whose own folder is named like a skipped folder, such as `crates/data`, is still skipped.

Run the tests from the repository root with `python3 -I tests/test_audit_scan.py` and `python3 -I tests/test_check.py`. The second uses stub commands, so it needs no Rust toolchain.

## How it was tested

Every case ran twice from the same prompt, once following the skill and once without it. Separate grading agents compared the results blind, against written rubrics or answer keys, and verified every audit finding against the code. The skill was revised after each round, only where a run showed a specific failure.

- **Writing and changing code:**
  - On a parser written from a spec and on a planted bug, the first version tied the baseline. It also caused three defects of its own: an error type callers could not branch on, an unneeded test dependency, and needless modules. Those rules were rewritten, and re-tests on two models came out clean.
  - On two real fixes the skill scored slightly higher: 11.5 against 11 of 12, and 8.5 against 8 of 9. On the first, the grader still preferred the baseline's Unicode-correct text search. The rule on Unicode case folding was added because of it.
- **Auditing planted and historical bugs:** on a crate with 18 planted defects, and on three real RustSec advisories checked out before their fixes, the skill matched the baseline. Both found every serious defect and all three advisories.
- **Auditing real code:** on alacritty_terminal 0.25.0, findings were scored against eight bugs fixed upstream after that release. Two runs without the skill found 5 of 8 each. Two runs with the final skill found 5 and 4, and one of them had the widest coverage of any run. Every finding in every report checked out. On a medium-sized PDF accessibility library, an earlier version of the skill found two small files that crash the whole application, which no other run found.

Limits: each version ran once or twice per case, and the two final alacritty runs differed by more than the gap between using the skill and not. The graders are the same model family as the agents they graded. No person who writes Rust has reviewed the rules yet. For an audit that matters, run it twice and merge the findings.

## Sources

The rules synthesize these sources. Where they disagree, `references/precedence.md` records the decision and why.

- [Microsoft Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/)
- [The Rust Style Guide](https://doc.rust-lang.org/style-guide/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Google's Rust style guide
- The Rust instructions in [awesome-copilot](https://github.com/github/awesome-copilot)

These crates were studied as examples of the practices in use:

- [memchr](https://github.com/BurntSushi/memchr)
- [jiff](https://github.com/BurntSushi/jiff)
- [ripgrep](https://github.com/BurntSushi/ripgrep)
- [fd](https://github.com/sharkdp/fd)
- [dua-cli](https://github.com/Byron/dua-cli)

## License

MIT. See `LICENSE`.
