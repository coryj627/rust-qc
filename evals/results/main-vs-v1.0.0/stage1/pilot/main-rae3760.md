# Audit report: alacritty_terminal 0.25.0

## Verdict

The terminal-emulation core works on ordinary input, and the existing test suite passes. But escape sequences, resizes and selections can still panic it, hang it, or corrupt the screen, and there are two paths to undefined behaviour from safe code. Defects: 3 Blocker, 10 Major, 0 Minor. There are also 6 QA hygiene items. Nothing in the repository or its output addressed me.

## Blocker

### Pushing too many keyboard modes panics or empties the title stack
Location: src/term/mod.rs:1295-1296 (`push_keyboard_mode`).
Problem: once the keyboard-mode stack reaches 4096 entries, the code removes from `title_stack` instead of `keyboard_mode_stack`. If the title stack is empty, the process panics. If it isn't, saved titles are lost and the keyboard stack keeps growing without limit. Any program that writes to the PTY can trigger this when `kitty_keyboard` is enabled.
Cause: a copy-paste slip: `self.title_stack.remove(0)`.
Evidence (ran): with `Config { kitty_keyboard: true }`, feeding `ESC [ > 1 u` 4097 times panics: "removal index (is 0) should be < len (is 0)" at term/mod.rs:1296.
Fix: `self.keyboard_mode_stack.remove(0)`, plus a regression test that pushes past the limit.

### `Row::new(0)` writes through a dangling pointer
Location: src/grid/row.rs:37-53. Reachable through the public `grid::Row::new` and through `Term::new` with 0 columns.
Problem: in release builds, a zero-column row writes one `T` into a zero-capacity `Vec`.
Cause: the only guard is `debug_assert!(columns >= 1)`, and the loop always performs the final `ptr::write`. There is no SAFETY comment.
Evidence (ran): `cargo +nightly miri run --release --bin row0` with `Row::<Cell>::new(0)` reports "Undefined Behavior: memory access failed: attempting to access 24 bytes, but got 0x8[noalloc] which is a dangling pointer" at row.rs:50.
Fix: replace the unsafe block with `inner.resize_with(columns, T::default)`. Alternatively, make `columns >= 1` a hard `assert!` and enforce it in `Term::new` and `Term::resize`.

### `getpwuid_r` output is assumed initialised before success is checked
Location: src/tty/unix.rs:77-88 (`get_pw_entry`, called from `tty::new`).
Problem: `entry.assume_init()` runs before `status` and `res` are checked. When the UID has no passwd entry (common in containers that run with an arbitrary UID) or the 1024-byte buffer is too small, `entry` is never written, and treating it as initialised is undefined behaviour. In addition, `status < 0` can never be true: `getpwuid_r` reports errors as positive errno values, so ERANGE is misreported as "pw not found".
Cause: wrong ordering, and a wrong error convention.
Evidence (read): lines 80, 82 and 86. Miri cannot run this FFI path.
Fix: check `status != 0` and `res.is_null()` first, and only then call `assume_init()`. Retry with a larger buffer on ERANGE.

## Major

### DCH with a large count erases cells to the left of the cursor
Location: src/term/mod.rs:1544-1563 (`delete_chars`).
Problem: `CSI n P` with n at or above the remaining width clears the whole line, not just the cursor onwards. Users lose on-screen text.
Cause: `count` is clamped to `columns`, not to `columns - cursor.column`. The clear loop then starts at `columns - count`.
Evidence (ran): on a 10-column terminal holding "0123456789", `ESC[1;6H ESC[20P` leaves "          ". It should leave "01234     ".
Fix: `let count = cmp::min(count, columns - cursor.point.column.0);`.

### ED 1 leaves the first line uncleared when the cursor is on line 2
Location: src/term/mod.rs:1761 (`clear_screen`, `ClearMode::Above`).
Problem: "erase above" from the second row does not clear row 0.
Cause: the guard `if cursor.line > 1` should be `> 0`; `reset_region(..Line(1))` would clear row 0 correctly.
Evidence (ran): with 4 lines of text, `ESC[2;3H ESC[1J` leaves line 0 as "aaaaa".
Fix: change the condition to `cursor.line > 0`, or drop it, since `reset_region(..Line(0))` is empty.

### Resizing to one column with a wide character loops forever and exhausts memory
Location: src/grid/resize.rs:271-363 (`shrink_columns`), reached from `Term::resize`.
Problem: `resize` never returns, and memory grows until the process is killed.
Cause: a wide character cannot fit in one column. Each pass moves it to a new row and leaves a leading spacer behind, so `row.shrink(1)` always produces more wrapped cells.
Evidence (ran): on a 4×2 terminal, writing "中" and then calling `resize(TermSize::new(1, 2))` reaches 505 MiB RSS after 0.52 s inside `resize`, in a release build. A seeded fuzzer first hit this as an OOM kill.
Fix: reject or clamp widths below 2 in `Term::resize` and `Term::new`, and document the minimum. Alternatively, replace a wide character with a blank when it cannot fit.

### A wide character in a one-column terminal panics
Location: src/term/mod.rs:1127-1129 (`input`).
Problem: after wrapping, the spacer is written at column 1 of a 1-column row.
Cause: `point.column += 1` runs without checking that a second column exists.
Evidence (ran): on a 1×2 terminal, input "中" panics with "index out of bounds: the len is 1 but the index is 1" at grid/mod.rs:439.
Fix: same minimum-width rule as the previous finding, or skip wide glyphs when `columns < 2`.

### A block selection along the last column panics in `selection_to_string`
Location: src/selection.rs:378-380 (`range_block`), panicking at src/term/mod.rs:584 (`line_to_string`).
Problem: when the normalised block start is on the right side of the last column, `start.column += 1` produces `columns`, which is then used as an index.
Cause: unlike `range_simple` (lines 352-355), `range_block` does not handle the right edge.
Evidence (ran): the fuzzer found this in 13 seeds, at every width from 1 to 11. Seed 123 is a 2-column terminal: `Selection::new(Block, (-1, 1), Left)`, then `update((-4, 1), Right)`, then `selection_to_string()`. It panics with "index out of bounds: the len is 2 but the index is 2".
Fix: clamp `start.point.column` to `last_column` in `range_block`, or treat a start past the edge as an empty block.

### Backward regex search can hang
Location: src/term/search.rs:317-430 (`regex_search_internal`) and `next_match_left`.
Problem: `search_next(.., Direction::Left, Side::Right, None)` with regex `a+b|中` never returns on a grid that contains wide characters and was reflowed from 7 to 10 columns. The same call searching right returns at once.
Cause (hypothesis, not traced): termination depends on reaching exactly `end` (`point == end || done`). `skip_fullwidth` takes a second unchecked `iter.prev()` over a `LEADING_WIDE_CHAR_SPACER` (line 464), which can step over `end`, so the wrap-around continues forever.
Evidence (ran): in tmp/probe/src/bin/search_hang.rs, a 7×6 terminal is fed `radgwnr中nsycdpd ESC[26067a ESC[4h xp ESC[?J CR ESC[H faj中rhboywl ESC[63584a ESC[?6h`, resized to 10×2, fed `zivm ESC[?1049l`, then searched left from (0, 1). It hangs (3 s watchdog) with history sizes 0, 5 and 10.
Fix: set `done` whenever a skipped cell equals `end`, or compare positions with `>=`/`<=` in search order, not with equality.

### Library calls `process::exit` and logs in the forked child and on resize failure
Location: src/tty/unix.rs:30-38 (`die!`), 53 (inside `pre_exec`) and 404 (`on_resize`).
Problem: if `TIOCSWINSZ` fails, `on_resize` terminates the host process, and a library should return the error. Inside `pre_exec`, `die!` logs (which takes a lock and allocates) and then runs exit handlers in the child. That is not async-signal-safe, and it can deadlock if another thread held the logger lock at fork time. `Error::new` with a string (line 240) and `env::set_current_dir` also allocate in the child.
Evidence (read): the lines above.
Fix: in `pre_exec`, return `Err(io::Error::last_os_error())` and use `libc::chdir` with a `CString` prepared before forking. Have `on_resize` log and continue, or change the trait to return `io::Result`.

### A non-UTF-8 passwd field panics even when the environment variable is set
Location: src/tty/unix.rs:91-97, 130.
Problem: `get_pw_entry` runs unconditionally, then calls `to_str().unwrap()` on `pw_name`, `pw_dir` and `pw_shell` and `assert_eq!`s the UID. A non-UTF-8 home directory in passwd crashes `tty::new` even when `HOME` is set.
Evidence (read): lines 91-97. `from_env` calls the function before checking `USER`, `HOME` or `SHELL`.
Fix: look up passwd lazily, and only when a variable is missing. Return `io::Error` (or use `OsStr`) instead of unwrapping.

### SIGHUP can go to a reaped, reused PID
Location: src/tty/unix.rs:296-307 (`Drop for Pty`).
Problem: `next_child_event` reaps the child through `try_wait` (line 382). `Drop` then sends `kill(child.id(), SIGHUP)` unconditionally. If the PID has been reused by then, an unrelated process gets the signal.
Evidence (read): lines 300 and 382.
Fix: track whether `try_wait` has returned `Some`, and skip `kill` once it has. Alternatively, signal through a pidfd.

### Unsafe blocks without SAFETY justification: 17
Location: for example src/grid/storage.rs:159 (`Storage::swap`), src/term/mod.rs:2334 (`TabStops::clear_all`), src/grid/row.rs:43, and src/tty/unix.rs:43, 235, 277, 299, 426.
Problem: `Storage::swap` reinterprets `Row<T>` as four `usize` words and relies only on a `debug_assert_eq!` of the size. `clear_all` uses `write_bytes` on `bool` where `fill(false)` would do the same. Neither block states why it is sound. The unsafe `fn`s `register` and `set_nonblocking` have no `# Safety` sections.
Evidence (ran): the `--audit` clippy pass reports "unsafe block missing a safety comment" 17 times in this crate.
Fix: add SAFETY comments. Replace `clear_all` with `self.tabs.fill(false)`, and `Storage::swap` with `self.inner.swap(a, b)` unless a benchmark justifies the unsafe version.

## QA hygiene

### Rustdoc fails with warnings denied: 3 errors
These are an unresolved link to `Mode` (term/mod.rs:2272) and unescaped `Row<T>` (grid/storage.rs:145 and 147) in private-item docs. Fix: write `ansi::Mode` in the link and backtick `Row<T>`.

### Documentation gaps: about 120
Missing docs on 38 methods, 26 fields, 14 associated functions, 13 structs and 13 variants. There are 10 missing `# Errors` sections and 4 missing `# Panics` sections, for example on `tty::new` and `Term::new` (which panics or worse on 0 columns).

### Public types without `Debug`: 16
For example `Term`, `RenderableContent`, `Pty`, and `TabStops`.

### Lint and convention items
- 14 assertions without messages.
- 9 `unwrap()` calls outside tests, for example tty/unix.rs:95-97 (also reported above).
- Glob re-exports at tty/mod.rs:13 and 18.
- A `#[allow]` without a reason at row.rs:117.
- `Error::new(ErrorKind::Other, ..)` could be `Error::other` (3 instances).

### ASCII-art diagram in a comment
grid/mod.rs:84-104 draws the grid layout with box characters. Describe the layout in sentences instead.

### Toolchain drift
`cargo clippy -D warnings` fails only in the sibling crate `alacritty_config_derive` (`unnecessary_map_or`, `double_ended_iterator_last`), because of lints newer than the project's toolchain. rustfmt was skipped: rustfmt.toml uses nightly-only options, and nightly rustfmt is not installed.

## Unverified

- The fuzzer, with search disabled, also tripped `debug_assert!(positive < self.len)` at grid/storage.rs:225 in seed 1545. That means some row index lands outside storage; in release builds this may read the wrong row or panic. I did not trace the cause.
- `ToWinsize` (tty/unix.rs:417-421) truncates `num_cols` and `num_lines` to `u16` and multiplies in `u16`. This only overflows at extreme sizes, and I did not test it.

## Verification commands

All were run from `crate/` unless noted.
- `cargo fmt --all --check`: skipped. It needs nightly rustfmt, which is not installed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: fail, only in alacritty_config_derive (toolchain drift).
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: fail, 3 errors in alacritty_terminal.
- `cargo test --workspace --all-features`: pass. alacritty_terminal: 128 unit tests and 44 ref tests.
- From `tmp/probe` (a throwaway crate with a path dependency), `cargo run --bin probe -- dch ed1 kitty wide1`: reproduces the DCH, ED 1, keyboard-mode and one-column wide-character findings.
- `cargo run --release --bin resize1`: reproduces the one-column resize runaway.
- `cargo run --bin search_hang -- 0 left`: reproduces the search hang.
- `NOSEARCH=1 cargo run --bin fuzz -- 4000`: 13 distinct panic sites, including the block-selection panic.
- `cargo +nightly-2026-10-01 miri run --release --bin row0`: reports the `Row::new(0)` undefined behaviour.
- I did not run Miri on the crate's own test suite, for lack of time.

## Closing summary

I covered the escape-sequence handlers in term/mod.rs, grid scrolling and storage, resize reflow, selection ranges, the Unix PTY code, and the gate. A seeded fuzzer drove sequences, resizes, selection and search through the public API, with Miri on the unsafe row constructor. Left unchecked: the Windows tty code, event_loop.rs, vi_mode.rs, most of search.rs beyond the hang (its cause is only a hypothesis), the storage index assertion, and a Miri run of the full test suite. I stopped at about 30 of 45 minutes to leave time for the report, while the fuzzer was still turning up new failures. More fuzzing time would probably find more.

Last clock reading: 29 minutes used of 45.