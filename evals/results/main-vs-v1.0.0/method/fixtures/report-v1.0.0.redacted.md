# Audit of examplecrate 1.2.0

## Summary

Two real defects and one minor one. 

## High

**Heap overflow when the iterator outruns its size hint**
src/lib.rs:1044 (insert_many).
the length is set to 0 as a panic guard, so reserve(1) sees spare capacity and never grows; later writes go past the allocation. I ran `cargo miri test` in <workspace>/tmp/probe and it reports an out-of-bounds write.

## Medium

**Parser accepts a truncated header**
src/parse.rs:88 (Header::read).
a 3-byte input returns Ok with a zero length instead of an error. I only read the code.

## Low

**Missing Debug on public Config**
src/config.rs:12.
