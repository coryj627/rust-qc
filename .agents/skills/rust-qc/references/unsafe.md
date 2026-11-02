# Unsafe code

**UNSAFE-1. Every `unsafe` block has a `// SAFETY:` comment immediately above it** that names each precondition of the operation and says why it holds at this point. memchr has more safety notes (361) than unsafe sites (333); that ratio is the standard.

```rust
// SAFETY: `i < self.len` is checked by the loop condition, and `self.ptr`
// points to an allocation of `self.len` initialized elements.
let item = unsafe { &*self.ptr.add(i) };
```

**UNSAFE-2. Every `unsafe fn` and `unsafe trait` has a `# Safety` doc section** listing every condition the caller or implementer must uphold. A `_unchecked` function documents what its checked counterpart verifies.

**UNSAFE-3. `unsafe` marks undefined-behavior risk only.** Do not mark a function `unsafe` because it is dangerous in some other way (deletes data, is slow).

**UNSAFE-4. Valid reasons are limited to three:** FFI and platform calls, a performance need proven by a benchmark, and a novel abstraction with no established safe alternative. Never use ad hoc `unsafe` to shorten safe code, transmute enums, bypass `Send`/`Sync` with `unsafe impl`, or extend lifetimes.

**UNSAFE-5. Encapsulate.** Wrap unsafe operations in the smallest possible safe abstraction, ideally in its own module, with business logic outside. The safe API must be sound for every possible safe caller, including callers passing closures that panic and trait impls (`Clone`, `Deref`, `Drop`, `Hash`, `Ord`) that misbehave.

**UNSAFE-6. Keep blocks minimal.** One unsafe operation per block where practical, so each SAFETY comment covers exactly what it justifies. Enable `unsafe_op_in_unsafe_fn` so the body of an `unsafe fn` still needs explicit blocks.

**UNSAFE-7. Test under Miri.** Run `cargo miri test` in CI for crates with unsafe code, as memchr does, and include adversarial test cases. Skip slow property tests under Miri with `#[cfg_attr(miri, ignore)]`.

**UNSAFE-8. Crates without unsafe say so.** Add `#![forbid(unsafe_code)]` (or `deny`, if a single module must opt out with a reasoned `expect`). dua-cli forbids it in its binary and denies it in its library.

**UNSAFE-9. FFI translates, it does not decide.** Business logic lives in a safe core crate; the FFI layer converts types and errors only. Prefer established binding crates over hand-written `extern` blocks, and document which call patterns are permitted.
