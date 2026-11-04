# Audit of examplecrate 1.2.0

## Summary

One High and one Medium defect, plus 2 other notes items. 

## High

**Heap overflow when the iterator outruns its size hint**
src/lib.rs:1044 (insert_many).
later writes go past the allocation. Any .filter() iterator triggers it.
the length is set to 0 as a panic guard, so reserve(1) sees spare capacity and never grows.
`cargo miri test --test probe insert_many_filter` in <workspace>/tmp reports an out-of-bounds write.

## Medium

**Parser accepts a truncated header**
src/parse.rs:88 (Header::read).
a 3-byte input returns Ok with a zero length instead of an error.
hypothesis: the bounds check compares against the declared length, not the buffer length.
src/parse.rs:90-94 slice the buffer before any length check.

## Other notes

**Public fallible functions without an Errors section: 20**

## Other notes

**Possible overflow in Grid::resize**
src/grid.rs:210.
rows * cols is computed in u16 and could wrap for very large grids; I could not build an input large enough within the memory limit.
