# Projects for answer-key evaluation

The 10 projects chosen for issue #2, with 76 candidate key bugs. Each project is tracked in its own issue, which builds the key and runs the evaluation.

## The list

Size is total lines / code lines / `unsafe` count in the audited path at the baseline, measured from the tag. Code lines exclude blank and comment-only lines. Inline test modules are counted.

| Crate | Issue | Baseline | Domain | Size | Key | Kinds |
|---|---|---|---|---|---|---|
| h2 | #12 | `v0.4.12` (2025-08-06) | Async network service and client | 25,593 / 18,009 / 1 | 8 | crash 1, resource 3, wrong 4 |
| quinn-proto | #13 | `quinn-proto-0.11.13` (2025-08-27) | Network protocol, second network project (sans-IO, not async) | 29,024 / 21,959 / 5 | 8 | crash 3, resource 3, wrong 2 |
| just | #14 | `1.50.0` (2026-04-19) | CLI tool | 22,954 / 19,936 / 3 | 8 | crash 3, resource 2, wrong 3 |
| heapless | #15 | `v0.9.1` (2025-08-20) | Embedded / `no_std` | 17,537 / 10,339 / 254 | 7 | soundness 6, wrong 1 |
| git2 | #16 | `git2-0.20.2` (2025-05-05) | FFI binding | 26,952 / 19,270 / 936 | 8 | crash 3, soundness 4, wrong 1 |
| mlua | #17 | `v0.11.4` (2025-09-28) | FFI binding, heavy `unsafe` | 20,923 / 15,133 / 472 | 8 | crash 2, soundness 2, resource 1, wrong 3 |
| zip | #18 | `v5.1.1` (2025-09-11) | Parser / file format | 13,256 / 10,315 / 24 | 8 | crash 2, resource 2, wrong 4 |
| pulldown-cmark | #19 | `v0.13.0` (2025-02-12) | Parser / file format | 13,648 / 11,398 / 4 | 8 | crash 1, resource 4, wrong 3 |
| moka | #20 | `v0.12.11` (2025-09-23) | Concurrency / data structure | 34,259 / 21,757 / 138 | 5 | soundness 2, resource 1, wrong 2 |
| simd-json | #21 | `v0.16.0` (2025-09-11) | Heavy `unsafe`, parser | 26,677 / 21,575 / 492 | 8 | crash 1, soundness 2, resource 1, wrong 4 |

Kinds: crash = crash or panic from input; soundness = undefined behaviour from safe code; resource = resource exhaustion; wrong = wrong behaviour.

## How far each fact was verified

- **Checked by script against the git history, for all 76 entries:** the commit exists, the baseline tag is its ancestor, it is not in the tag, it touches the audited Rust source, and every quoted baseline line matches its file and line number exactly. Sizes and dates were measured from the tags.
- **Baseline code and fix diff read in the coordinating session, for 19 of 76, marked ✓:** all 11 entries the removed-lines check flagged, plus 8 sampled. All 19 held up. The flags came from later refactors moving the code.
- **Read by a vetting agent only, for the other 57:** the quoted lines are proven to exist, but the judgement that they show the bug is the agent's.
- **Not confirmed for any project:** nothing was built or run, so no bug has been reproduced at its baseline. 'Builds and tests on stable Rust in a Linux container' comes from reading manifests and CI files only.
- **AI policy:** 'none found' means none in the repo's contributor docs and `.github/` at the branch tip, plus the org's `.github` repo where one exists. It does not mean the project has no policy.

## Projects

### h2

- **Repository:** [hyperium/h2](https://github.com/hyperium/h2), audited path `src/`
- **Baseline:** `v0.4.12`, tagged 2025-08-06, commit `41a0f805c8`
- **Domain:** Async network service and client
- **Size:** 25,593 total lines, 18,009 code lines, 1 `unsafe`
- **Shortfalls:** No soundness (1 `unsafe`). Only one crash. Lockfile absent at the tag.
- **AI or LLM policy:** None found (repo docs and org `.github`).
- **Build notes (from reading, not built):** From reading only, nothing built. Cargo.toml and .github/workflows/CI.yml at v0.4.12: no build.rs seen in the listing, no system library needed by the crate itself (CI installs libssl-dev, apparently for dev/examples or h2spec tooling). No required features; `unstable` only widens the API and is needed for frame-level access used by test support (h2-support/h2-tests). Stable Rust works: CI matrix is nightly, beta and stable, MSRV 1.63. Tests: `cargo test` for lib/doc tests and `cargo test -p h2-tests` for integration tests, which use an in-memory mock IO from tests/h2-support, so no network. `ci/h2spec.sh` is the only external conformance step and is optional. Workspace members include tests/h2-fuzz, tests/h2-tests, tests/h2-support, util/genfixture, util/genhuff. No Cargo.lock at the tag, so dependencies resolve fresh unless pinned. Should be testable on stable in a Linux container.
- **Lockfile at the tag:** no; pin dependencies when building the key

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`193833e87c`](https://github.com/hyperium/h2/commit/193833e87c639e39751f339a9c375a44c8bfab54) | resource | Empty or tiny DATA frames are queued without limit when the stream is not drained; flow control does not charge empty frames, so memory grows unbounded | RUSTSEC-2026-0258 / PR #935 / CHANGELOG 0.4.16 | yes | The advisory's 'panic if the length overflows' was not reproduced; the unbounded queue is what the baseline shows. |
| [`4f51fff882`](https://github.com/hyperium/h2/commit/4f51fff8826f533a098da6fc3b7437e2d11b1f4a) ✓ | crash | A request with more than 24,576 unique header fields panics in HeaderMap::append when no small header-list limit is set, crashing on one remote HEADERS block | PR #909 / CHANGELOG 0.4.15 | yes |  |
| [`7c97e8917b`](https://github.com/hyperium/h2/commit/7c97e8917b7fd9a09741e49016c9220393570d0d) | resource | flush busy-loops at 100% CPU when the transport's poll_write returns Ok(0), because zero-byte writes are retried forever | PR #921 / CHANGELOG 0.4.16 | yes | Triggered by a transport returning `Ok(0)`, not by a remote peer. |
| [`a26b42746a`](https://github.com/hyperium/h2/commit/a26b42746ae5c69fa32bb9411de3b5e7440cd165) ✓ | resource | Reserved push-promise streams are not counted against any limit, so a server can make a client hold unbounded stream state by withholding response HEADERS | PR #968 / CHANGELOG 0.4.20 | yes | Upstream deliberately departs from the RFC here. A TODO at the baseline points straight at it. |
| [`d96d34751d`](https://github.com/hyperium/h2/commit/d96d34751d79a19f8caa44b4565ceb451c719052) | wrong | Conflicting duplicate content-length headers are accepted; only the first value is used, a request-smuggling style framing error | PR #967 / CHANGELOG 0.4.20 | yes |  |
| [`3c5f61c9ba`](https://github.com/hyperium/h2/commit/3c5f61c9bac3b9ab9b24031a3bb90d8b58eccb9a) | wrong | A response with no :status pseudo-header is accepted and surfaced to the client as 200 OK instead of being a stream error | Issue #958 / PR #959 / CHANGELOG 0.4.20 | yes |  |
| [`f5f159471a`](https://github.com/hyperium/h2/commit/f5f159471ae187c6c90cd1e78924bafb3aaa436d) | wrong | On receiving GOAWAY, streams above last_stream_id are closed regardless of initiator, so a client's GOAWAY(0) kills the server's in-progress responses on client-initiated streams | Issue #885 / PR #886 / CHANGELOG 0.4.14 | yes | Needs RFC 9113 knowledge to spot. |
| [`d9689ea53b`](https://github.com/hyperium/h2/commit/d9689ea53be6702a2c9f3c644101f282344edff2) | wrong | DATA received after local GOAWAY on a stream above max_stream_id is dropped without connection flow-control accounting, leaking window until lower streams stall forever | PR #895 / CHANGELOG 0.4.14 | yes |  |

### quinn-proto

- **Repository:** [quinn-rs/quinn](https://github.com/quinn-rs/quinn), audited path `quinn-proto/src/`
- **Baseline:** `quinn-proto-0.11.13`, tagged 2025-08-27, commit `b2b930a066`
- **Domain:** Network protocol, second network project (sans-IO, not async)
- **Size:** 29,024 total lines, 21,959 code lines, 5 `unsafe`
- **Shortfalls:** No soundness (5 `unsafe`). Not itself async. Three fixes have no regression test in the commit.
- **AI or LLM policy:** **Has one.** `CONTRIBUTING.md`, 'AI policy': using LLMs as coding tools is welcome; comments to maintainers must be written by humans.
- **Build notes (from reading, not built):** From reading only, nothing built. Baseline note: suggested tag quinn-0.11.9 and quinn-proto-0.11.13 are the same commit b2b930a; the crate's own tag is used. Workspace members: quinn, quinn-proto, quinn-udp, bench, perf, fuzz, docs/book (fuzz and docs/book not default members). Cargo.lock is committed and CI uses `cargo test --locked`. Default features: rustls-ring, log, bloom; ring needs a C compiler. Alternative: --no-default-features --features rustls-aws-lc-rs. No nightly needed: CI matrix is stable and beta, MSRV 1.74.1. quinn-proto tests appear to run client and server in-process through the state machine, so no network expected (from reading). Scope tests with `-p quinn-proto`. Should be testable on stable in a Linux container.
- **Lockfile at the tag:** yes

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`655a8ad094`](https://github.com/quinn-rs/quinn/commit/655a8ad094e4fad463c90c4666c62db7de56384b) | crash | A truncated max_datagram_frame_size or min_ack_delay transport parameter panics on unwrap while parsing the peer's handshake parameters | RUSTSEC-2026-0037 / CVE-2026-31812 / GHSA-6xvm-j4wr-6v98 | no | SHA is the main-line commit; the advisory cites a PR on the 0.11.x branch, matched by subject and diff. |
| [`981d88eccb`](https://github.com/quinn-rs/quinn/commit/981d88eccb36ae5e8585bcd3145debb00969a462) | crash | Zero-length STREAM frames in unordered-read mode insert an empty range into RangeSet, breaking its invariant so later overlapping frames panic the connection | RUSTSEC-2026-0338 / GHSA-465w-v9q3-7j98 / PR #2868 | yes | Needs unordered reads. |
| [`6912dad550`](https://github.com/quinn-rs/quinn/commit/6912dad55028a47e652e915bcc6caf2a40575b06) | crash | A peer's large min_ack_delay at low RTT makes Duration::clamp panic with min greater than max when ACK_FREQUENCY sending is enabled | RUSTSEC-2026-0340 / GHSA-3g6h-r5qx-f6hq | yes | Needs `ack_frequency_config`. |
| [`fed0321a9a`](https://github.com/quinn-rs/quinn/commit/fed0321a9a672819662caab37f5662f1ad91308e) ✓ | resource | Out-of-order stream fragments with many gaps make the assembler buffer unbounded chunks, so a peer can pin far more memory than its flow-control window suggests | RUSTSEC-2026-0185 / CVE-2026-25800 / GHSA-4w2j-m93h-cj5j / PR #2694 | no | SHA is the main-line commit; the advisory cites a PR on the 0.11.x branch, matched by subject and diff. |
| [`77a74a9290`](https://github.com/quinn-rs/quinn/commit/77a74a92901dd2ecae60eeab35b8f76a8e01c030) | resource | NEW_CONNECTION_ID frames for already-retired sequence numbers push onto pending retire_cids with no limit or dedup, growing memory without bound while the peer withholds ACKs | RUSTSEC-2026-0339 / GHSA-hmxj-32vh-65vr / PR #2790 | no | SHA is the main-line commit; the advisory cites a PR on the 0.11.x branch, matched by subject and diff. |
| [`a3109c3413`](https://github.com/quinn-rs/quinn/commit/a3109c3413015ab1b0d9a3c3477e1bc2fa915149) ✓ | resource | Empty DATAGRAM frames are charged zero bytes, never trigger eviction, and queue without limit, so a peer can grow the receiver's heap until OOM | RUSTSEC-2026-0342 / GHSA-2hv7-gw8g-gpq5 / PR #2790 | yes | SHA is the main-line commit; the advisory cites a PR on the 0.11.x branch, matched by subject and diff. |
| [`621e38abbc`](https://github.com/quinn-rs/quinn/commit/621e38abbccdfe8d664d50b517fccec3316fe658) | wrong | Transport parameter values are not checked to occupy their declared length, so surplus bytes are parsed as further parameters and a peer can smuggle in parameters it never encoded | Commit message 'reject transport parameters with a mismatched length' | yes |  |
| [`10ba697060`](https://github.com/quinn-rs/quinn/commit/10ba697060dc86ecf11c1656ef3ee214c3357cd3) | wrong | RESET_STREAM on a stopped stream credits connection flow control again for data already credited by stop(), inflating the local connection window | Commit message 'avoid duplicate flow-control credit for stopped streams' | yes |  |

### just

- **Repository:** [casey/just](https://github.com/casey/just), audited path `src/`
- **Baseline:** `1.50.0`, tagged 2026-04-19, commit `05c630394a`
- **Domain:** CLI tool
- **Size:** 22,954 total lines, 19,936 code lines, 3 `unsafe`
- **Shortfalls:** No soundness (3 `unsafe`). Both resource bugs sit behind `set unstable` or `set lazy`.
- **AI or LLM policy:** None found. The README's one LLM mention is about a feature, not contributions.
- **Build notes (from reading, not built):** From reading Cargo.toml, build.rs and .github/workflows/ci.yaml at 1.50.0; nothing was built. Workspace: members [".", "crates/*"], with autotests = false in the root package. rust-version 1.85.0, edition 2024. No system libraries in the manifest; build.rs only sets a Windows stack size. No crate features needed. CI runs `cargo test --all` on stable on Linux, macOS and Windows. Nightly appears only for fuzz and udeps recipes in the justfile. The test job also runs www/install.sh, which needs network, but that is separate from `cargo test`. The tests spawn shell commands, so bash and coreutils should be present (inferred from the workflow removing a broken WSL bash on Windows, not confirmed). Should work on stable Rust in a Linux container.
- **Lockfile at the tag:** yes

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`53a658c69a`](https://github.com/casey/just/commit/53a658c69a38e9e35c513d92b3190d170c24236e) | resource | A self-recursive user-defined function (`foo() := foo()`) recurses without limit and overflows the stack, aborting the process instead of reporting an error | CHANGELOG 1.51.0: Fail on deep recursion instead of overflowing stack, PR #3319 | yes | Needs `set unstable`. A stack overflow is also a crash, so the kind is arguable. |
| [`b05deedbe7`](https://github.com/casey/just/commit/b05deedbe72dc346d664bada472166e84c02cae4) | resource | Dependency traversal for `set lazy` has no visited set, so a diamond-shaped chain of N recipes takes about 2^N steps and effectively hangs | CHANGELOG 1.56.0: Avoid exponential blowup when traversing dependencies, PR #3546 | yes | Needs `set lazy` at this tag; upstream's test does not set it. |
| [`b31c0fa028`](https://github.com/casey/just/commit/b31c0fa0284d76b0c5e9cd179be301e80a6266e6) | crash | `datetime('%!')` panics: chrono's formatter errors on an invalid format string and to_string() panics on that error | CHANGELOG 1.51.0: Don't panic on invalid datetime format string, PR #3304 | yes |  |
| [`0360c3f78a`](https://github.com/casey/just/commit/0360c3f78aaedef0ef6015b5eda8ce9896d9a695) | crash | `mod foo '/outside/dir'` with both `justfile` and `.justfile` there panics while building the ambiguity error | CHANGELOG 1.57.0: Avoid panic in ambiguous module file error, PR #3638 | yes |  |
| [`29c3ed5774`](https://github.com/casey/just/commit/29c3ed5774f94b915e713a33b1eaca60b6ae7216) ✓ | crash | An empty module path reaches resolve_path, where components.len() - 1 underflows and --show or --usage panics | CHANGELOG 1.55.1: Fix panic when empty path is passed to --show and --usage, PR #3511 | yes | Route `--show ' '` confirmed by reading. Upstream's `--show .` was not traced at this tag. |
| [`0beb79f49d`](https://github.com/casey/just/commit/0beb79f49def4245757cddf15d52e485332ecb49) | wrong | `just --dry-run` still executes shell() function calls, so a dry run has side effects, while backticks are correctly skipped | CHANGELOG 1.56.0: Do not run shell() commands during dry run, PR #3588 | yes |  |
| [`044a01c960`](https://github.com/casey/just/commit/044a01c9609a4ac1a8be8b556445706f5b084b51) | wrong | `set fallback` discards every error from searching parent directories, so an ambiguous parent justfile is reported as an unknown recipe | CHANGELOG 1.57.0: Return search errors other than NotFound during fallback, PR #3617 | yes |  |
| [`c2b9a8c8ef`](https://github.com/casey/just/commit/c2b9a8c8ef1bd8d08754a3666b90d920047c981e) | wrong | Command-line overrides (`just x=1`) are ignored inside user-defined functions, which re-evaluate the assignment and see the original value | CHANGELOG 1.51.0: Fix overrides not being visible in user-defined functions, PR #3307 | yes | Needs `set unstable`. |

### heapless

- **Repository:** [rust-embedded/heapless](https://github.com/rust-embedded/heapless), audited path `src/`
- **Baseline:** `v0.9.1`, tagged 2025-08-20, commit `19188229ac`
- **Domain:** Embedded / `no_std`
- **Size:** 17,537 total lines, 10,339 code lines, 254 `unsafe`
- **Shortfalls:** **Falls short: only 2 of 4 kinds, and 6 of 7 entries are soundness.** No crash or resource bug survived; the one hang candidate was a regression from after the tag. Good for testing `unsafe` auditing only.
- **AI or LLM policy:** None found (repo docs and org `.github`).
- **Build notes (from reading, not built):** From reading Cargo.toml, build.rs and .github/workflows/build.yml at v0.9.1; nothing was built. No system libraries. CI test command on stable: `cargo test --features="alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes"`. Unit tests need no nightly. Nightly is used for the Miri job (`cargo miri test`, MIRIFLAGS=-Zmiri-ignore-leaks) and the TSan job (`-Zbuild-std`); Miri is the natural way to exercise most candidates, so a nightly toolchain with the miri component is recommended though not required, since the double-drop and out-of-bounds cases can also be asserted on stable. build.rs probes target capabilities by running rustc on small programs; no network. cfail/ is a separate trybuild crate. Single crate, no workspace, no rust-version in Cargo.toml at this tag. Should work on stable Rust in a Linux container.
- **Lockfile at the tag:** no; pin dependencies when building the key

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`e458255075`](https://github.com/rust-embedded/heapless/commit/e458255075e81b3bfdd41f72bc1eb085573c4af5) | soundness | Deque::clear and HistoryBuf::clear drop contents before resetting state, so a panicking Drop leaves the state stale and later drops or reads cause use-after-free | CHANGELOG v0.9.3: Fixed unsoundness in Deque::clear, HistoryBuf::clear and IndexMap::clear with panicking drop; issue #646 | yes | Same panic-safety pattern as e2ecc4e and 27e7ad3. |
| [`e2ecc4e7dc`](https://github.com/rust-embedded/heapless/commit/e2ecc4e7dc54e991e5429f3c145651bf4a2d99f1) | soundness | HistoryBuf::write drops the old element in place before overwriting it, so a panicking Drop leaves a dropped element in the buffer, which is dropped again | CHANGELOG: Fixed unsoundness in HistoryBuf::write with panicking drop; issue #659 | yes | Same panic-safety pattern as e458255 and 27e7ad3. |
| [`27e7ad3eaa`](https://github.com/rust-embedded/heapless/commit/27e7ad3eaa54fa65f38e1796aa67b69fcb138f18) | soundness | vec::IntoIter::drop resets len only after dropping the remaining elements, so a panicking Drop makes the inner Vec drop everything again, including moved-out items | CHANGELOG: Fixed unsoundness in Vec::IntoIter::drop with panicking drop; issue #659 | yes | Same panic-safety pattern as e458255 and e2ecc4e. |
| [`d987e5c24c`](https://github.com/rust-embedded/heapless/commit/d987e5c24c1abd1cfe2f2d20ed74c0456837b878) | soundness | Deque::make_contiguous sets back to len on a full deque, so back == N; after pop_front, push_back writes one slot past the buffer | CHANGELOG: Fixed Deque::make_contigous leading to an inconsistent state; issue #676, PR #677 | yes |  |
| [`2e73f81ee4`](https://github.com/rust-embedded/heapless/commit/2e73f81ee4090d3ef7fe2d2d95a37d509bc597b2) ✓ | soundness | IndexMap Pos packs hash and index into a NonZeroU32 assuming {0xFFFF, 0xFFFF} never occurs, and N is unbounded, so a full 65536-entry map hits new_unchecked(0), and larger N truncates indices | CHANGELOG: Fixed unsoundness in IndexMap::insert, limited size to u16::MAX + 1; issue #672, PR #673 | yes |  |
| [`906f8ae525`](https://github.com/rust-embedded/heapless/commit/906f8ae525fc8f019886f5fb39d9878557611de5) | wrong | IndexMap::truncate clears removed index slots without repairing probe chains, so keys still in the map become unreachable through get while iter() still shows them | CHANGELOG: Fixed IndexMap::truncate leading to an inconsistent state; issue #674, PR #675 | yes |  |
| [`c70d081ec2`](https://github.com/rust-embedded/heapless/commit/c70d081ec2140687b76d51be418d89d4a5845a90) | soundness | pool::UnionNode has no #[repr(C)] although Box and Arc pools cast its pointer directly to the data pointer, relying on an unguaranteed field offset of 0 | CHANGELOG (filed under v0.9.1 but the commit is after the tag): Fixed the memory layout of UnionNode; issue #605, PR #606 | no | WEAK: no visible failure on current compilers; found by reading only. Consider dropping. |

### git2

- **Repository:** [rust-lang/git2-rs](https://github.com/rust-lang/git2-rs), audited path `src/`
- **Baseline:** `git2-0.20.2`, tagged 2025-05-05, commit `1fb5f64819`
- **Domain:** FFI binding
- **Size:** 26,952 total lines, 19,270 code lines, 936 `unsafe`
- **Shortfalls:** No resource bug (work happens in C). Soundness entries share a null-pointer pattern, so one insight may score several.
- **AI or LLM policy:** None found (repo docs and org `.github`).
- **Build notes (from reading, not built):** From reading Cargo.toml, libgit2-sys/build.rs and .github/workflows/main.yml at the tag; not built. Needs a C toolchain: libgit2-sys compiles the vendored libgit2 with cc unless a matching system libgit2 is found. The libgit2 source is a git submodule (libgit2-sys/libgit2), so a checkout must init submodules (CI uses submodules: true), or use the crates.io package. Default features are ssh and https, which pull in libssh2-sys and openssl-sys (system OpenSSL dev headers unless vendored-openssl); `cargo test --no-default-features` avoids them, and CI runs both that and plain `cargo test` on ubuntu stable, plus beta and nightly. Nothing requires nightly. No bug in this key needs ssh or https; the Remote::list regression test uses a file:// remote. Workspace root is the git2 crate, with members systest and git2-curl; test with `-p git2` to skip those. Cargo.lock is committed. Testable on stable Rust in a Linux container, with network only for crates.io and the submodule, as far as reading shows.
- **Lockfile at the tag:** yes

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`d38a2da67d`](https://github.com/rust-lang/git2-rs/commit/d38a2da67d66ae7f546d0306a07dfec84cfe846b) | soundness | Dereferencing a new or default Buf passes a null pointer to slice::from_raw_parts and from_raw_parts_mut, which is undefined behaviour from safe code | RUSTSEC-2026-0008; CHANGELOG 0.20.4 Fixed (#1213) | yes |  |
| [`241e62928a`](https://github.com/rust-lang/git2-rs/commit/241e62928ac45c8593b4b385f09e78e9aeba703d) | soundness | Remote::list passes a null pointer to slice::from_raw_parts when the remote advertises no refs, which is undefined behaviour and errors or aborts in debug builds | RUSTSEC-2026-0183; CHANGELOG 0.21.0 Fixed (#1250); issue #1217 | yes | That libgit2 returns null comes from the advisory or commit; the C source was not read. |
| [`2eafc457a4`](https://github.com/rust-lang/git2-rs/commit/2eafc457a4c59a7eae03846fede70b44a2042635) ✓ | soundness | BlameHunk::final_signature and orig_signature wrap possibly null signature pointers from buffer blames, so reading the Signature dereferences null | RUSTSEC-2026-0184; CHANGELOG 0.21.0 Changed (#1254); issue #1253 | yes | That libgit2 returns null comes from the advisory or commit; the C source was not read. |
| [`09269c6b4c`](https://github.com/rust-lang/git2-rs/commit/09269c6b4c292253054719f7bf968c1a100ca7be) | soundness | Remote::get_refspec returns a Refspec tied to the repository lifetime instead of the Remote, so holding it past the Remote drop is a use-after-free from safe code | CHANGELOG Unreleased Changed (#1287); issue #1286 | yes |  |
| [`f016ecc6ba`](https://github.com/rust-lang/git2-rs/commit/f016ecc6ba941111aadcc2027c03f16860e4a1b9) | wrong | MergeOptions::skip_reuc sets GIT_MERGE_FAIL_ON_CONFLICT instead of GIT_MERGE_SKIP_REUC, so it makes merges fail on conflict and does not skip REUC | CHANGELOG 0.21.0 Fixed (#1194) | no |  |
| [`2027edb9e7`](https://github.com/rust-lang/git2-rs/commit/2027edb9e712ccc980c148b7e7fe3d0551aebac5) | crash | ReferenceNames iterator unwraps str::from_utf8, so a repository with a non-UTF-8 ref name panics during iteration | CHANGELOG 0.21.0 Fixed (#1239); issue #1175 | yes |  |
| [`ae77eced6f`](https://github.com/rust-lang/git2-rs/commit/ae77eced6f1621ad99f536b9d753f4a95f41ff81) | crash | Index::add_all, update_all and remove_all crash when given a callback and an empty pathspec, because the callback runs CStr::from_ptr on a null matched_pathspec | CHANGELOG Unreleased Fixed (#1267, #1273); follow-up 4f2e641 | yes | That libgit2 returns null comes from the advisory or commit; the C source was not read. Also undefined behaviour. |
| [`2e3054e715`](https://github.com/rust-lang/git2-rs/commit/2e3054e7156090c8627ead29d8fb43494c5c5c5a) ✓ | crash | Repository::submodules asserts git_submodule_lookup succeeded inside an extern C callback, so a lookup failure panics and crashes instead of returning an error | CHANGELOG 0.21.0 Fixed (#1220) | no | Only reachable through a race with another git process, so it can be read but not exercised reliably. |

### mlua

- **Repository:** [mlua-rs/mlua](https://github.com/mlua-rs/mlua), audited path `src/`
- **Baseline:** `v0.11.4`, tagged 2025-09-28, commit `247208edb1`
- **Domain:** FFI binding, heavy `unsafe`
- **Size:** 20,923 total lines, 15,133 code lines, 472 `unsafe`
- **Shortfalls:** Most fixes landed in one September 2026 batch after heavy refactors, and many have no linked issue.
- **AI or LLM policy:** None found.
- **Build notes (from reading, not built):** From reading Cargo.toml and .github/workflows/main.yml at the tag; not built. A Lua version feature must be selected or the crate will not build usefully: use lua54 plus vendored, which compiles Lua from source with cc (C compiler needed, no system Lua). CI on ubuntu runs `cargo test --features "lua54,vendored"` and `"lua54,vendored,async,serde,macros,anyhow,userdata-wrappers"`, plus the same with send, on stable and nightly; a nightly-only job runs tests under AddressSanitizer. For this key use `lua54,vendored,async,serde,macros`; the serde feature is required for two entries. Declared MSRV 1.79. Workspace members mlua_derive and mlua-sys. Dev-dependencies are heavy (hyper, reqwest, criterion, tokio) but tests in this key need no network beyond crates.io, as far as reading shows; trybuild UI tests are in #[ignore] runs. Testable on stable Rust in a Linux container.
- **Lockfile at the tag:** no; pin dependencies when building the key

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`98cbcd3d3d`](https://github.com/mlua-rs/mlua/commit/98cbcd3d3de7cebaad4fa9456fb338ff21474292) | resource | With detect_mixed_tables, a table with few entries but a huge integer key is encoded as an array up to that key, so a one-entry table can expand to billions of nulls | Commit subject: serde: Prevent excessive array expansion for sparse tables; CHANGELOG v0.12.2 has no specific entry | yes | Needs the opt-in `detect_mixed_tables` option. |
| [`6ec314ac2e`](https://github.com/mlua-rs/mlua/commit/6ec314ac2e9ec9484d66a6370681677ae78cc3bf) ✓ | crash | Serde serialization and deserialization of Lua tables have no depth limit, so a script-built deeply nested table overflows the Rust stack and aborts the process | CHANGELOG v0.12.2: Added recursion_limit to SerializeOptions and DeserializeOptions | yes |  |
| [`afe1d837a6`](https://github.com/mlua-rs/mlua/commit/afe1d837a66cef263bcd81a36daab3663ba19991) ✓ | crash | Table::raw_set with a nil or NaN key calls lua_rawset unprotected on the fast path, so the Lua error aborts the process instead of returning Err | Commit message: Return Lua errors for nil and NaN keys instead of aborting in fast path | yes | The fix edits code refactored after the tag; the equivalent baseline code was read directly. |
| [`08e2663624`](https://github.com/mlua-rs/mlua/commit/08e2663624c79329bc8e2da6895725320aae71a9) | soundness | Scoped userdata methods that escape the scope still read the moved-out userdata because of a pointer match, and scoped values created during scope teardown are never invalidated, both giving use-after-free from safe code | CHANGELOG v0.12.2: Prevent scoped userdata and callbacks from outliving their scope during cleanup | yes |  |
| [`abe1c5478c`](https://github.com/mlua-rs/mlua/commit/abe1c5478ce15cc8bd2e9c71820061997509271c) | soundness | Lua::create_thread and Thread::reset accept a Function from a different Lua state and read its index from this state's reference thread, reading the wrong slot or an out-of-range stack index | Commit subject: Validate function ownership before thread creation/reset | yes |  |
| [`752955cb7e`](https://github.com/mlua-rs/mlua/commit/752955cb7ecb5c020a6c94e0b536752769676cbc) | wrong | Table == slice ignores extra slice elements, so a table compares equal to any longer slice with the same prefix, and an empty table equals every slice | Commit subject: Match lengths when comparing tables with slices | yes |  |
| [`b2bdc4562e`](https://github.com/mlua-rs/mlua/commit/b2bdc4562ec509f6c19339f5bd93954463fdbc6b) | wrong | HashSet and BTreeSet of integers fail to round-trip: into_lua makes {[1]=true,...}, but from_lua sees positive raw_len and reads the true values as elements | Commit subject: Fix HashSet/BTreeSet integer round trip conversion | yes |  |
| [`3771a4d7f8`](https://github.com/mlua-rs/mlua/commit/3771a4d7f8727eccae9a3590750b5657214cb189) | wrong | Value equality casts Integer to f64, so large integers compare equal to floats that differ in value, such as i64::MAX == 2^63 as f64 | CHANGELOG v0.12.2: Fix precision loss when comparing integer and floating point Values | yes |  |

### zip

- **Repository:** [zip-rs/zip2](https://github.com/zip-rs/zip2), audited path `src/`
- **Baseline:** `v5.1.1`, tagged 2025-09-11, commit `6423fee132`
- **Domain:** Parser / file format
- **Size:** 13,256 total lines, 10,315 code lines, 24 `unsafe`
- **Shortfalls:** No soundness. Security fixes cluster in `extract`, so reading it closely may score several at once.
- **AI or LLM policy:** None found, though 126 of 734 commits since the baseline disclose an AI co-author.
- **Build notes (from reading, not built):** From reading Cargo.toml and .github/workflows/ci.yaml at v5.1.1, not from building. Single crate; fuzz_read/ and fuzz_write/ are separate manifests, not workspace members. MSRV 1.83, edition 2021. Use --all-features: legacy-zip (needed for the Shrink, Reduce and Implode candidates) is not a default feature. Default features pull compression crates including zstd (0.13, wraps C zstd), so a C compiler is probably needed; no system library is named in the CI apt steps for the test job. CI runs `cargo test --all <flags>` on stable, nightly and MSRV across Ubuntu, macOS and Windows, so stable in a Linux container should work. Nightly is only used for miri, clippy, rustfmt, minimal-versions and AFL fuzzing. Tests read fixtures under tests/data and use tempfile; no network or external service seen. Dev-dependency clap is pinned to =4.4.18.
- **Lockfile at the tag:** no; pin dependencies when building the key

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`a543a1fc8f`](https://github.com/zip-rs/zip2/commit/a543a1fc8f3f3eef36f27bb137230295284cdb98) ✓ | crash | extract() reserves a symlink buffer from the attacker-declared uncompressed size, so a tiny ZIP64 archive aborts the process on allocation failure | RUSTSEC-2026-0429 / GHSA-q7v9-x93c-xqv3 / PR #984 | yes |  |
| [`3a273aa7b6`](https://github.com/zip-rs/zip2/commit/3a273aa7b6215966fadb29d3776b256b663da089) ✓ | wrong | extract() keeps SUID, SGID and sticky bits from external attributes, so a root extraction can create setuid files | RUSTSEC-2026-0367 / GHSA-vv5m-jxxh-4627 | yes |  |
| [`09c030eab5`](https://github.com/zip-rs/zip2/commit/09c030eab56e8db8fb3c1c33f554407b30bad8fa) ✓ | wrong | extract() creates symlinks whose targets leave the destination, and an absolute target through an intermediate symlink passes the lexical containment check, allowing writes outside the destination | RUSTSEC-2026-0368 / GHSA-4v9h-p7m9-rh26 (the commit message describes the same defect; the advisory does not name the commit, so the mapping is inferred) | yes | Mapping to the advisory is inferred. Fix commit is co-authored by Claude Opus 5. |
| [`71efbd8391`](https://github.com/zip-rs/zip2/commit/71efbd8391d4b371a23cc22a65d406ac85764597) ✓ | resource | Entry reads are not bounded by the declared uncompressed size, so an entry declaring 10 bytes can inflate to 200 MB and defeat decompressed_size() checks | CHANGELOG 9.0.1 / PR #1022 | yes |  |
| [`25bb3de95d`](https://github.com/zip-rs/zip2/commit/25bb3de95dcde89767238581005b38b652acf4d9) | resource | An empty archive's EOCD can claim a central directory offset far past the EOCD, and new_append then writes there, growing the output to GiBs | CHANGELOG 9.0.1 / PR #1025 | yes |  |
| [`8b4991288b`](https://github.com/zip-rs/zip2/commit/8b4991288b79ad11cce201c2d785d5baa795bd3e) | crash | Shrink, Reduce and Implode decoders reserve the declared size, so an entry declaring 0xFFFFFFFFFFFFFFFF panics with capacity overflow from the read API | CHANGELOG 9.0.0 / PR #991 | yes | Needs the non-default `legacy-zip` feature. |
| [`fd7c9b1ec5`](https://github.com/zip-rs/zip2/commit/fd7c9b1ec5c06107ffbce435a30478d8ba19a41c) | wrong | Shrink decoder never assigns LZW code 8191, so valid entries whose table fills fail with 'invalid code' | CHANGELOG 9.0.1 / PR #1029 | yes | Needs the non-default `legacy-zip` feature. |
| [`53bdadce59`](https://github.com/zip-rs/zip2/commit/53bdadce59cc3440f19bd6527499554bf72d0b7c) ✓ | wrong | ZipStreamReader::visit never calls visit_additional_metadata and swallows the parse error, so stream extraction silently never applies Unix permissions | CHANGELOG 9.0.0 / PR #1030 | yes |  |

### pulldown-cmark

- **Repository:** [pulldown-cmark/pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark), audited path `pulldown-cmark/src/`
- **Baseline:** `v0.13.0`, tagged 2025-02-12, commit `f17d98ae9e`
- **Domain:** Parser / file format
- **Size:** 13,648 total lines, 11,398 code lines, 4 `unsafe`
- **Shortfalls:** No soundness (`unsafe` is SIMD only). 4 of 8 need `ENABLE_WIKILINKS`, off by default, including the only crash.
- **AI or LLM policy:** None found.
- **Build notes (from reading, not built):** From reading Cargo.toml, pulldown-cmark/Cargo.toml and .github/workflows/rust.yml at v0.13.0, not from building. Workspace with members bench, dos-fuzzer, fuzz, pulldown-cmark and pulldown-cmark-escape; default-members are the last two, so test with `cargo test -p pulldown-cmark`. Default features are getopts and html. pulldown-cmark/build.rs exists (gen-tests feature regenerates spec tests; checked-in suite files exist). CI uses ubuntu-latest and runs `cargo test`, `--workspace`, `--features=simd,gen-tests`, `--features=serde` and `--no-default-features`; rust-version 1.71.1, no nightly required by those jobs. Release profile sets panic=abort. No system libraries, network or external services seen. Unsafe is SIMD-only: lib.rs forbids unsafe_code without the simd feature. Wikilink candidates need Options::ENABLE_WIKILINKS at parse time, not a cargo feature.
- **Lockfile at the tag:** yes

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`967d260d4c`](https://github.com/pulldown-cmark/pulldown-cmark/commit/967d260d4c309eca8b7ad1242e0295da5897fb32) | resource | A wikilink like [[a\|]] leaks its child's next chain into following content, so event count doubles per link and a few KB can exhaust memory | RUSTSEC-2026-0414 / issue #1179 / PR #1102 | yes | Needs `ENABLE_WIKILINKS`. |
| [`67b946977e`](https://github.com/pulldown-cmark/pulldown-cmark/commit/67b946977e010764e04bc2cf4d2c07cb9b39526c) | resource | Nested ![[![[...]]]] image wikilinks each rescan their whole body, giving quadratic parse time | RUSTSEC-2026-0433 / PR #1186 | yes | Needs `ENABLE_WIKILINKS`. |
| [`ebf31da886`](https://github.com/pulldown-cmark/pulldown-cmark/commit/ebf31da886d90f34d9519278e323dbefb08444af) ✓ | crash | handle_wikilink underflows end_ix - start_ix and slices an inverted range on input like ![[] ]()]], panicking | issue #1108 (OSS-Fuzz) / PR #1111 | yes | Needs `ENABLE_WIKILINKS`. Trigger traced by reading, not run. |
| [`ccc4706c28`](https://github.com/pulldown-cmark/pulldown-cmark/commit/ccc4706c289cfaec72b87be1b0716f50dc05960e) ✓ | resource | Footnote interrupt probe UTF-8 validates all remaining input per line, so [^ on every line is quadratic | issue #1076 / PR #1136 | no |  |
| [`38b7e8a0da`](https://github.com/pulldown-cmark/pulldown-cmark/commit/38b7e8a0da1f71e9baf01ec409288486f9c695ab) | resource | Deeply nested lists followed by blank lines are quadratic because scan_containers walks the whole spine per blank line; N=80,000 took about 34 s | commit message (with timings) / cmark PR 520 / bench added in 366a6e54 | no |  |
| [`061b03acba`](https://github.com/pulldown-cmark/pulldown-cmark/commit/061b03acbaea7a9d49ab52c733712e01b3b0c8d6) | wrong | A tab-indented definition subtracts column-based indent from a byte offset, so into_offset_iter ranges can fall inside a multibyte character and panic when slicing | issue #1142 / PR #1143 | yes |  |
| [`520bd63b77`](https://github.com/pulldown-cmark/pulldown-cmark/commit/520bd63b77b547e36b7af28eaf8f9c86119f8356) | wrong | Wikilink end offset stops one byte short of the closing ]], so into_offset_iter reports a wrong span and later text overlaps | issue #1030 / commit message | yes | Needs `ENABLE_WIKILINKS`. |
| [`0b7c9f2ee4`](https://github.com/pulldown-cmark/pulldown-cmark/commit/0b7c9f2ee493f0202fcea44a48a639f5896cb246) | wrong | HTML block end tags match case-sensitively, so <script> closed by </SCRIPT> never ends and following Markdown becomes raw HTML | commit message / CommonMark 4.6 start condition 1 / regression spec added | yes |  |

### moka

- **Repository:** [moka-rs/moka](https://github.com/moka-rs/moka), audited path `src/`
- **Baseline:** `v0.12.11`, tagged 2025-09-23, commit `9f166f2a12`
- **Domain:** Concurrency / data structure
- **Size:** 34,259 total lines, 21,757 code lines, 138 `unsafe`
- **Shortfalls:** **Weakest key: only 5 entries, and over the size range by total lines (34,259).** No crash. Three entries are races and one is visible only in Miri.
- **AI or LLM policy:** None found. `CLAUDE.md` and `.claude/` exist at the branch tip but are tooling, not policy, and are absent at the baseline.
- **Build notes (from reading, not built):** From reading, not building. No system libraries; build.rs is a no-op unless cfg(rustver). Nothing is enabled by default, so features are required: `cargo test --features sync` and `--features future` (CI also runs 'future, sync, logging'). CI tests stable, beta, MSRV 1.70 and nightly on ubuntu-latest; .ci_extras version pins apply only to the MSRV and nightly jobs. Should work on stable in a Linux container, but crates.io access is needed for dev-dependencies, and the MSRV may need the pins. Miri (nightly, -Zmiri-tree-borrows), Loom (--cfg moka_loom), Kani and Trybuild have their own workflows. Some doc examples mention reqwest; whether any doctest performs network I/O is unconfirmed. Single crate, no workspace. Races are timing dependent, so long stress runs may be needed.
- **Lockfile at the tag:** no; pin dependencies when building the key

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`6290c58626`](https://github.com/moka-rs/moka/commit/6290c58626caed2f147605c1f8dce1d0ca0d90db) | resource | Stale WriteOp::Upsert admits an entry already gone from the hash table, leaving an orphan deque node at the LRU front; eviction stalls forever and the cache grows past max_capacity | CHANGELOG 0.12.16 Fixed; PR #592, issue #590 (introduced in v0.12.0) | yes | Race; needs a stress run to show. |
| [`876015b1d5`](https://github.com/moka-rs/moka/commit/876015b1d577dbb6cd0d5c509030fb1a2a362324) | wrong | future::Cache::and_compute_with removes its waiter before writing to the cache, so a concurrent call on the same key reads the old value and an update is silently lost | CHANGELOG 0.12.14 Fixed; PR #574 | yes | Race; needs a stress run to show. |
| [`5d4c748929`](https://github.com/moka-rs/moka/commit/5d4c748929d6d749bfe3eed24424763e560e756f) ✓ | soundness | Timer wheel frees a popped node whose expiration was cleared without clearing the entry's timer_node pointer; later housekeeping uses the dangling pointer (use-after-free, or the unreachable! panic in unlink_timer) | CHANGELOG 0.12.13 Fixed; PR #566, issues #565, #527, #524 | no | Race, needs a custom `Expiry`; not exercised. |
| [`2e053649f8`](https://github.com/moka-rs/moka/commit/2e053649f8c388ec0f4c132167c20ece6d815b9e) | soundness | Deque::move_to_back keeps a pointer derived from a temporary &mut to the node, which later accesses invalidate; Miri reports UB under Tree Borrows | CHANGELOG 0.12.12 Fixed; PR #553 | no | Miri Tree Borrows report only, with no visible failure outside Miri. |
| [`34a21d3ed4`](https://github.com/moka-rs/moka/commit/34a21d3ed4a8b4c81397c737c165e8e55c1f248b) | wrong | When expire_after_update returns None for an already expired entry, the expiration is not cleared, so the updated entry stays expired and unreadable | CHANGELOG 0.12.13 Fixed; PR #564, issue #561 (future::Cache counterpart is 816418f, PR #549) | yes |  |

### simd-json

- **Repository:** [simd-lite/simd-json](https://github.com/simd-lite/simd-json), audited path `src/`
- **Baseline:** `v0.16.0`, tagged 2025-09-11, commit `7e4603e6e4`
- **Domain:** Heavy `unsafe`, parser
- **Size:** 26,677 total lines, 21,575 code lines, 492 `unsafe`
- **Shortfalls:** 3 of 8 fixes are co-authored by Claude, the family under test. Several came from differential fuzzing, so they may suit fuzzing more than review.
- **AI or LLM policy:** None found, though 7 of 64 commits since the baseline disclose an AI co-author.
- **Build notes (from reading, not built):** From reading, not building. No system libraries and no build.rs. Edition 2024, rust-version 1.88; CI tests run on 1.88 stable and nightly across Linux, Windows and macOS. Default features: swar-number-parsing, serde_impl, runtime-detection. Other features: value-no-dup-keys, 128bit, known-key, approx-number-parsing, big-int-as-float, ordered-float. The `portable` feature is commented out at this tag. Testable on stable in a Linux container with crates.io access; no external services. The profile sets panic = abort for release, so catch_unwind tests must use the test or dev profile. CI sets RUSTFLAGS=-C target-cpu=native with variations disabling avx2, pclmulqdq and sse4.2. .cargo/config.toml only sets wasm flags. fuzz/ is a separate crate. Test data lives in data/. Linux x86_64 reach: AVX2 and SSE4.2 paths are what real runs use. Native code is reached by Miri (as PR #445 reports) or by an emulated CPU; I could not confirm whether target-feature flags change the runtime detection. Only acabe0c among the keys depends on this; the others are in shared code.
- **Lockfile at the tag:** no; pin dependencies when building the key

| Fix commit | Kind | What was wrong | Source | Test | Caveat |
|---|---|---|---|---|---|
| [`abb59c3beb`](https://github.com/simd-lite/simd-json/commit/abb59c3bebf8cde1f0b974133a960ab3a968891d) | crash | No nesting limit: deeply nested JSON overflows the stack in the recursive tape-to-Value and serde paths and aborts the process | Issue #473; commit 'Introduce max depth checking' | yes |  |
| [`f56f1df6f3`](https://github.com/simd-lite/simd-json/commit/f56f1df6f3d79b0f7e8566734071cfd66149c40f) ✓ | soundness | serde number parsing uses the unchecked next_(), so reusing an exhausted Deserializer from safe code reads past the end of the tape (UB in release, index panic in debug) | Issue #471 (includes a Miri reproduction); commit 'fix #471' | yes |  |
| [`acabe0c13e`](https://github.com/simd-lite/simd-json/commit/acabe0c13e98424ace8a11ae9b1248ccb6e84025) | soundness | With runtime-detection and no AVX2 or SSE4.2, the native fallback skips UTF-8 validation entirely, so invalid UTF-8 becomes a &str | PR #445 ('Found by running miri on jsonchecker fail34') | no | Needs Miri or an emulated CPU on Linux x86_64. |
| [`5d030ac77e`](https://github.com/simd-lite/simd-json/commit/5d030ac77eeaf0a23727bb1f2eef0def197cbb49) | resource | Repeated object keys are inserted without deduplication into a hash-backed map, giving quadratic time on crafted input in to_owned_value and to_borrowed_value | Issue #491; commit 'don't write duplicate keys into the hashmap when decoding' | yes | Timing not reproduced at this tag; `halfbrown` 0.3 read through the GitHub API. Crate docs call duplicate keys undefined for `Value`. |
| [`538e8b2d4d`](https://github.com/simd-lite/simd-json/commit/538e8b2d4d7b744238ec3864c9ab474c4cc13202) | wrong | Tape Value::get_array, get_object, try_get_array and try_get_object slice the tape one node short, dropping the last element of a nested container | Commit 'fix bug in tape parsing'; regression also pinned by PR #474 (22609d9) | yes |  |
| [`b215cf0ceb`](https://github.com/simd-lite/simd-json/commit/b215cf0cebae2d8a8dd8bdff05c0db3c929fb9e3) | wrong | Huge exponents wrap through `as i32` and give wrong finite numbers (1e4294967296 parses as 1.0), while valid very long exponents such as 0e99999999999 are rejected | PR #486 | yes | Fix commit is co-authored by Claude. |
| [`bd1ccfad79`](https://github.com/simd-lite/simd-json/commit/bd1ccfad79ea4ff2f89310fad483365c3f450d04) | wrong | Fixed-length targets leave unread array elements on the tape, which are then read as later values: from_slice::<((u8,u8),u8)>("[[1,2,3],4]") returns ((1,2),3) | PR #484 | yes | Fix commit is co-authored by Claude. |
| [`ff8f5c8bb7`](https://github.com/simd-lite/simd-json/commit/ff8f5c8bb75dfc56803321c2f5a92656c7de842f) | wrong | A lone high surrogate such as "\ud800" parses as NUL (U+0000) instead of returning an error | PR #482 | yes | Fix commit is co-authored by Claude Opus 5.5. |

## Alternates, vetted but not chosen

All counts come from the same checks as above.

- **russh** `v0.54.4` (18,751 total / 15,303 code): strong async SSH client and server, 8 entries, no soundness. Best swap for quinn-proto if you want a truly async second network project. 3 entries need the same direct read the others got. Its PR template asks contributors to declare AI involvement.
- **quick-xml** `v0.38.3` (31,261 total / 18,360 code): 8 entries, no soundness, over the range by total lines. Has an AI-assisted contributions policy, and 3 of 8 fixes are co-authored by Claude.
- **rusqlite** `v0.37.0` (21,012 total / 14,748 code): FFI, 8 entries, all four kinds, though its resource entry is only a small error-path leak. Swap for git2 if you prefer a full mix over RustSec-backed entries.
- **bat** `v0.25.0` (9,504 total / 7,621 code): CLI, 8 entries, only crash and wrong behaviour.

Rejected:

- **bumpalo:** a full key needs 2022's 3.11.0 (3.19.0 gives only 4 entries). Code lines at that tag (3,547) are below the floor, and the key includes the long-public RUSTSEC-2022-0078.
- **hashbrown:** 4 entries at a 2023 tag, two of which need direct review.
- **smoltcp:** the natural `no_std` network crate, but 52,314 total lines at `v0.12.0`, well over the range.

Excluded before vetting: memchr, jiff, ripgrep, fd and dua-cli, which the skill studied as examples, and alacritty_terminal, already used.

## Risks to the evaluation

1. **Model-family bias.** Disclosed AI co-authors appear on 4 of the 76 key fixes: 3 in simd-json and 1 in zip. If a Claude model found or fixed a bug, a Claude auditor may be more likely to find it. Only disclosed trailers can be counted. Consider scoring those entries separately.
2. **Training-data leakage.** All baselines are from 2025 or 2026 and the fixes came after them, which limits this but does not remove it for models with later cutoffs. Agents must get a workspace built from the tag, with no later history.
3. **Agent files.** No chosen baseline contains `AGENTS.md`, `CLAUDE.md` or similar, so the audit agents will not load project instructions. moka's tip does contain them, so build from the tag, not the tip.
4. **Unpinned dependencies.** h2, heapless, mlua, zip, moka and simd-json have no `Cargo.lock` at the tag. A fresh resolve can pull in later fixes or break the build, so pin by date when building each key.

## Key format

One YAML file per project. `confirmed_at_baseline` starts at `read` and moves to `reproduced` when the key-building step shows the bug in a container. `match_rule` is what lets graders score blind. Runs must name the skill version, since the README says earlier results predate versioning.

```yaml
project: h2
repo: hyperium/h2
audited_path: src/
baseline: {tag: v0.4.12, commit: 41a0f80, date: 2025-08-06}
size: {total_lines: 25593, code_lines: 18009, unsafe: 1}
build: {toolchain: stable, features: [], lockfile: pinned-by-date, notes: ""}
ai_policy: none found
bugs:
  - id: h2-1
    fix_commit: 193833e87c   # full SHA in the real file
    kind: resource-exhaustion
    summary: Empty DATA frames queue without limit when a stream is not drained.
    location: [src/proto/streams/recv.rs:675]
    source: RUSTSEC-2026-0258
    confirmed_at_baseline: read   # read | reproduced | unconfirmed
    match_rule: Names unbounded queueing of empty or tiny DATA frames in the receive path.
runs:
  - skill_version: v1.0.0   # required; "none" for the baseline arm
    arm: with-skill
    model: ""
    time_budget_minutes: 60
    report: ""
    bugs_found: []
```

## Follow-up issues

One per project. Each builds the key, reproduces each bug in a container, and runs the method from issue 2. That is the point where the build criterion and bug presence get tested for real, and an entry that fails to reproduce is dropped or replaced from the reserves named in the vetting data.

