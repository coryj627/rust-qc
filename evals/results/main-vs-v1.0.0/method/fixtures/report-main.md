# Audit of examplecrate 1.2.0

## Verdict

One Blocker and one Major defect, plus 2 QA hygiene items. Findings: 1 Blocker, 1 Major, 0 Minor.

## Blocker

### Heap overflow when the iterator outruns its size hint
Location: src/lib.rs:1044 (insert_many).
Problem: later writes go past the allocation. Any .filter() iterator triggers it.
Cause: the length is set to 0 as a panic guard, so reserve(1) sees spare capacity and never grows.
Evidence (ran): `cargo miri test --test probe insert_many_filter` in /srv/rqc/audits/projects/rf00001/tmp reports an out-of-bounds write.
Fix: restore the length before reserving.

## Major

### Parser accepts a truncated header
Location: src/parse.rs:88 (Header::read).
Problem: a 3-byte input returns Ok with a zero length instead of an error.
Cause: hypothesis: the bounds check compares against the declared length, not the buffer length.
Evidence (read): src/parse.rs:90-94 slice the buffer before any length check.
Fix: check the length before slicing.

## QA hygiene

### Public fallible functions without an Errors section: 20
Reported by check.sh --audit. Start with Store::open (src/store.rs:88).
Fix: add an Errors section to each.

## Unverified

### Possible overflow in Grid::resize
Location: src/grid.rs:210.
Problem: rows * cols is computed in u16 and could wrap for very large grids; I could not build an input large enough within the memory limit.

## Verification

- `cargo test --locked`: pass.
- `cargo +nightly-2026-10-01 miri test`: fail, see the Blocker.

## Closing summary

I covered the parser and the grid, and left the PTY code unchecked. This is why I stopped: the remaining time was too short for another probe.

2 findings. Clock: 44 minutes used of 45.
