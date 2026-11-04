I followed the rust-qc skill's audit workflow.

# Audit of examplecrate 1.2.0

## Verdict

Two real defects and one minor one. Findings: 1 Blocker, 1 Major, 1 Minor.

## Blocker

### Heap overflow when the iterator outruns its size hint
Location: src/lib.rs:1044 (insert_many).
Problem: the length is set to 0 as a panic guard, so reserve(1) sees spare capacity and never grows; later writes go past the allocation. I ran `cargo miri test` in /srv/rqc/audits/projects/rf00000/tmp/probe and it reports an out-of-bounds write.
Fix: restore the length before reserving.

## Major

### Parser accepts a truncated header
Location: src/parse.rs:88 (Header::read).
Problem: a 3-byte input returns Ok with a zero length instead of an error. I only read the code.
Fix: check the length before slicing.

## Minor

### Missing Debug on public Config
Location: src/config.rs:12.
Problem: the gate's clippy step flags missing_debug_implementations.
Fix: derive Debug.

## Verification

- `cargo test --locked`: pass.
- `cargo clippy --all-targets`: fail, 3 warnings.

3 findings. Clock: 41 minutes used of 45.
