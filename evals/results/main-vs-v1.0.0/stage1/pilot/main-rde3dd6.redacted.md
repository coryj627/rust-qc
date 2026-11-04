# Audit report: alacritty_terminal 0.25.0

## Summary

The escape-sequence handler is mostly correct. I compared about 25 control sequences against xterm, and fuzzed 75,000 random sequence-and-resize runs with no panic outside selection handling. But the crate has real defects. Output from a terminal program can make it panic, three public safe functions can cause undefined behaviour, and several editing and copy paths produce wrong text. The Unix PTY code also panics or exits the process when the operating system fails.

 There are 6 other notes items, counted separately.

## High

**Kitty keyboard-mode push panics after 4096 pushes**
src/term/mod.rs:1296 (push_keyboard_mode).
when the keyboard-mode stack is full, the code trims the title stack instead of the keyboard stack. Any program writing to the terminal can crash it when `kitty_keyboard` is on. If the title stack has entries, it silently loses the user's saved titles, and the keyboard stack grows without limit.
`self.title_stack.remove(0)` should be `self.keyboard_mode_stack.remove(0)`. With an empty title stack, `remove(0)` panics.
with `Config { kitty_keyboard: true, .. }`, feeding `"\x1b[>1u".repeat(4097)` panics at mod.rs:1296: "removal index (is 0) should be < len (is 0)".

**Out-of-range block selection panics when copied**
src/selection.rs:378 (range_block); the panic is at src/term/mod.rs:584 (line_to_string).
a block selection that starts on the right half of the last column returns a range with `start.column == columns`. `selection_to_string` then indexes past the end of the row. A user can trigger this with a block-selection mouse drag.
`start.point.column += 1` is not clamped to the last column. The simple-selection version wraps the column; the block version does not.
on a 4x3 terminal containing "abcd\r\nefgh", a Block selection from (0,3) Right to (1,3) Left gives `to_range` = start column 4, end column 2. `selection_to_string()` then panics: "index out of bounds: the len is 4 but the index is 4". The fuzzer found this 94 times in 60,000 runs.

**`Row::new(0)` writes through a dangling pointer**
src/grid/row.rs:43-53. Also reachable through `Grid::new` and `Term::new` with 0 columns.
undefined behaviour from safe public code. The only guard is a `debug_assert!`, which release builds remove.
the loop always does a final `ptr::write` into a `Vec` with capacity 0.
Miri, with debug assertions off, on `Row::<Cell>::new(0)` reports "Undefined Behavior occurred here" at row.rs:50.

**`Grid::scroll_up`/`scroll_down` write out of bounds with a large region**
src/grid/storage.rs:153-176 (Storage::swap), called from src/grid/mod.rs:191-307.
the public `Grid::scroll_down(&Range<Line>, usize)` can be reached through `Term::grid_mut()`. A region past the screen makes the unchecked swap read and write outside the row buffer. This is undefined behaviour from safe code.
`compute_index` only uses `debug_assert!` to check bounds, and `swap` uses raw `add`/`offset` without the bounds check that `Index` has. There is no SAFETY comment.
in a release build, `Grid::<Cell>::new(2, 2, 10).scroll_down(&(Line(0)..Line(50)), 1)` crashes the test with SIGSEGV.

**`get_pw_entry` assumes the passwd struct is initialised before checking the result**
src/tty/unix.rs:80.
`entry.assume_init()` runs before `status` and `res` are checked. When `getpwuid_r` finds no entry or fails, the struct is uninitialised, and assuming it is initialised is undefined behaviour. This is reached from the safe `tty::new`. A user ID with no passwd entry is common in containers.
the check order is wrong. Also, `status < 0` never detects an error, because `getpwuid_r` returns a positive errno.
lines 77-88.

## Medium

**DCH with a large count also erases cells left of the cursor**
src/term/mod.rs:1544 and 1560 (delete_chars).
`CSI 100 P` at column 6 of a 10-column line blanks the whole line, including the five cells before the cursor.
the count is clamped to `columns`, not to `columns - cursor_column`, and the clearing loop starts at `columns - count`.
"0123456789", then `ESC[1;6H ESC[100P`, gives "          ". The expected result is "01234     ".

**ED 1 (erase above) leaves the first line when the cursor is on line 2**
src/term/mod.rs:1761.
`CSI 1 J` with the cursor on the second row does not clear row 1.
the condition is `if cursor.line > 1`, so `reset_region(..Line(1))` never runs. It should be `> 0`.
"AAAAA\r\nBBBBB\r\nCCCCC", then `ESC[2;3H ESC[1J`, leaves row 0 as "AAAAA".

**Copying a line that ends in a wrapped wide character inserts a character from the line above**
src/term/mod.rs:629 (line_to_string).
when a selection ends on a line whose last cell is the leading spacer for a wrapped wide character, the copied text gets the first character of the previous line, not the wide character. On the topmost line in the history this goes out of range: the fuzzer hit the `positive < self.len` debug assertion in storage.rs:225.
`self.grid[line - 1i32][Column(0)]` should be `line + 1`, because the wide character wraps onto the next line.
on a 5-column terminal with "XYZ\r\nabcd中", selecting row 1 from column 0 to 4 copies "abcdX". The expected text is "abcd中".

**Back-tab ignores the pending wrap, so the next character jumps to the next line**
src/term/mod.rs:1567-1589 (move_backward_tabs); move_forward_tabs at 1592 has the same problem.
after a character is printed in the last column, `CSI Z` moves the cursor but leaves `input_needs_wrap` set. The next character then wraps instead of landing at the tab stop. Other cursor moves, such as CUB, clear the flag.
20x2 terminal, `ESC[1;20H y ESC[Z z`: "z" ends up at (1,0). The expected position is (0,16). The same test with `ESC[4D` passes.

**Code between fork and exec is not async-signal-safe**
src/tty/unix.rs:236-262 (the `pre_exec` closure), plus `set_controlling_terminal` at 42-55.
after fork, the closure:
- allocates in `env::set_current_dir`, which builds a CString, and in `Error::new` with a string payload;
- on an ioctl failure, logs through `log::error!` and calls `std::process::exit`, which takes locks, formats text and runs atexit handlers.

In a multi-threaded parent, this can deadlock the child.
the lines above.

**The library exits the process when a PTY resize fails**
src/tty/unix.rs:403-405 (on_resize, using `die!`).
a failed TIOCSWINSZ ioctl ends the whole host application with exit status 1.

**The library panics on passwd contents and system-call failures**
src/tty/unix.rs:91, 95-97 and 430.

- `to_str().unwrap()` panics on a passwd name, home or shell that is not UTF-8.
- This happens even when USER, HOME and SHELL are set, because `get_pw_entry` always runs.
- `assert_eq!(entry.pw_uid, uid)` and `assert_eq!(res, 0)` on `fcntl` turn environment failures into panics.

the lines above.

**SIGHUP can reach a reused process ID**
src/tty/unix.rs:296-307 (Drop for Pty), together with `try_wait` at 382.
`next_child_event` can reap the child through `try_wait`. `Drop` then calls `kill(child.id(), SIGHUP)` without checking that, so the signal can hit an unrelated process that reused the PID.

**The PTY event loop panics on a re-register failure**
src/event_loop.rs:316, and the ref-test writer at 150.
`reregister(...).unwrap()` turns a poller error into a panic of the PTY thread. The same applies to `write_all(...).unwrap()` when writing a ref-test recording.

**Unsafe blocks without SAFETY justification**
17 blocks. The ones that carry the soundness findings above are grid/storage.rs:159 and grid/row.rs:43. Others are term/mod.rs:2334 (TabStops::clear_all), tty/unix.rs:43, 76-97, 235, 277 and 299, and event_loop.rs:216.
none of them states why it is sound, and two of them are not sound (see the High findings). `TabStops::clear_all` uses `write_bytes` where the safe `self.tabs.fill(false)` does the same thing.

**Public `SelectionRange::new` panics on its input without a Panics section**
src/selection.rs:44.
`assert!(start <= end)` is reachable from caller input, but the documentation has no `# Panics` section and the assertion has no message.

## Low

**Wrong range in the colour index documentation**
src/term/color.rs:14.
the doc table says the grayscale ramp is "233..256". The colour cube ends at 232, so the ramp is 232..256.

**Test helpers are part of the public API**
src/term/mod.rs:2418 (`pub mod test`, including `mock_term` with `unwrap()`).
test scaffolding appears in the public API, so it is covered by semver.

## Other notes

**Rustdoc fails with `-D warnings`: 3 errors**
- term/mod.rs:2272: broken intra-doc link to `Mode`.
- grid/storage.rs:145 and 147: unescaped `Row<T>` is read as an HTML tag.

**Workspace clippy fails on the newer toolchain (toolchain drift)**
Two errors (`unnecessary_map_or`, `double_ended_iterator_last`) in `alacritty_config_derive`, which is outside this audit's scope. They stop `cargo clippy --workspace` before it reaches `alacritty_terminal`. The crate's MSRV is 1.74 and these lints are newer.

**Documentation gaps (advisory lints)**
- 115 missing-docs items: methods, fields, structs, variants, modules and others.
- 10 functions returning `Result` without an `# Errors` section, for example `RegexSearch::new` and `EventLoop::new`.
- 4 functions without a `# Panics` section.

**Missing `Debug` implementations: 16**
Examples: `EventLoop`, `Notifier`, `EventLoopSender`, `Colors`, `RenderableContent`.

**Asserts without messages (14) and `unwrap` in production code (9)**
For example `search.rs:293` (`start_state_forward(...).unwrap()`) and `event_loop.rs:221`. Most of these should be `expect` with the invariant stated, or `?`.

**Style and convention**
- `pub use self::unix::*` / `windows::*` glob re-exports at tty/mod.rs:13 and 18 hide the platform API surface.
- 4 TODO or XXX comments have no owner or issue.
- There is a box-drawing diagram in the `Grid` docs at grid/mod.rs:83-107.
- There are `#[allow]` attributes without a reason.

## Other notes

- `ToWinsize` (tty/unix.rs:420-421) and `text_area_size_pixels` (term/mod.rs:2259-2260) multiply `u16` values without checking. Very large windows could overflow, which panics in debug builds and wraps in release. I did not construct a case.
- The Windows code under `src/tty/windows` was not built or linted, because there is no Windows target here.
