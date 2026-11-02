# Errors and panics

## Choosing the mechanism

**ERR-1. `Option` for absence with an obvious cause.** `checked_add`, `HashMap::get`, `str::find`. The caller gains nothing from details.

**ERR-2. `Result` for recoverable failure the caller may handle.** Parsing, I/O, network, validation of external input. The error says which failure happened.

**ERR-3. Panic for detected programming bugs.** A violated invariant or broken precondition is a defect to fix, not a condition to handle. Do not return an error that no caller could act on. Prefer designing the panic away with types (correct by construction) over checking at runtime.

**ERR-4. Libraries do not panic on plausible caller input.** `parse(&str)` returns `Result`. Indexing with a caller-supplied index either returns `Option` (`get`) or panics with a documented `# Panics` section (`[]`), following the standard library's split.

**ERR-5. Never swallow errors.** No `let _ = fallible();` without a comment explaining why the failure is irrelevant. No `.ok()` that discards a `Result` silently. No `map_err(|_| ...)` that throws away the cause.

## Library error types

**ERR-6. Callers can branch on failure without parsing text.** Expose a concrete error type implementing `std::error::Error`, `Display`, and `Debug`, and give callers a typed way to tell failures apart: a `kind()` method returning a public `#[non_exhaustive]` enum, or `is_*()` predicates (`is_overflow()`, `is_not_found()`), plus accessors for useful context (`position()`, `path()`). Keeping fields private is right; a private kind with no accessor is a defect, because the only way left to react to a specific failure is to match on the message. A plain `#[non_exhaustive]` error enum is also fine when it has no extra context. jiff's `Error` in `crates/jiff/src/error/mod.rs` shows the private-internals, predicate-method form, with its design documented on the type.

**ERR-7. Scope error types to failure domains.** One crate-wide `Error` is fine when operations share failure modes and compose. Separate types (`ConfigError`, `DownloadError`) when they do not. Do not build one giant enum spanning unrelated operations.

**ERR-8. Preserve the cause.** Implement `source()` to return the underlying error. Convert upstream errors with `From` impls so `?` works, rather than scattering `map_err` at call sites.

**ERR-9. `Display` messages are lowercase, without trailing punctuation**, describe what failed, and point at the problem: the offending token and its position (`unknown unit "x" at byte 1`). Do not copy the whole input into the error: it allocates on every failure, and it puts possibly secret values (tokens, passwords in config) into logs. Do not repeat the source error's text when `source()` already returns it.

**ERR-10. `thiserror` is acceptable** to generate `Display`, `Error`, and `From` boilerplate. It must not push you into exposing every internal variant publicly.

**ERR-11. Backtraces for services.** Long-running services and applications debugged from logs capture `std::backtrace::Backtrace` when constructing errors. `no_std` and hot-path libraries skip it.

## Application errors

**ERR-12. Applications use `anyhow`** (fd, dua-cli, and ripgrep all do). Add context at each layer that knows something the lower layer does not: `.with_context(|| format!("failed to read config {}", path.display()))`. Print with `{:#}` to show the chain on one line.

**ERR-13. `main` maps errors to exit codes** in one place, as ripgrep does. Handle `BrokenPipe` as a successful exit for CLI tools whose output is commonly piped.

## Panics

**PANIC-1. No bare `unwrap()` in production code.** Use `?`, `if let`, `let ... else`, `match`, or `expect`.

**PANIC-2. `expect` messages state the invariant**, not the hope: `expect("regex literal is valid")`, `expect("queue is non-empty: checked by caller loop")`. Not `expect("failed")` or `expect("should work")`.

**PANIC-3. Pattern-match instead of check-then-unwrap.** `if let Some(x) = opt { ... }`, never `if opt.is_some() { opt.unwrap() }`.

**PANIC-4. Assertions carry context.** `assert!(refund > 0, "refund must be positive, got {refund}")`. The message adds information the condition does not already express.

**PANIC-5. Document every panic** reachable from a public function in `# Panics`.

**PANIC-6. No `todo!`, `unimplemented!`, `dbg!`, or stray `println!`** in committed library code. `unreachable!` is acceptable with a message explaining why the branch cannot happen.

**PANIC-7. No `catch_unwind` as control flow.** It is a last-resort isolation boundary (thread pools, FFI edges), never a substitute for `Result`, and it does nothing under `panic = "abort"`.

**PANIC-8. Never reach into a poisoned `Mutex`.** Propagate the panic (`lock().expect("mutex poisoned: a holder panicked")`) unless the protected data is provably valid at every point.
