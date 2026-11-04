The review found **14 actionable issues: 2 High, 11 Medium, and 1 Low**. The existing tests pass with the requested lint cap, but local regressions demonstrate incorrect parsing, malformed serialization, inconsistent APIs, and feature-build failures. An ordinary parsing test also fails under Miri.

The release source was left unchanged. Regression tests and logs remain in `/work/tmp`.

## High

### 1. NEON parsing performs a misaligned vector write

[src/impls/neon/stage1.rs:207](/work/crate/src/impls/neon/stage1.rs:207), write at line 226.

`flatten_bits` selects an aligned vector write when the element offset is divisible by four. That does not establish alignment of the underlying `Vec<u32>` allocation, which only guarantees four-byte alignment.

Executed evidence: Miri stopped while parsing the ordinary object `{"name":"caf\u00e9","items":[1,null,true,{"x":"line\nbreak"}]}` through `to_owned_value`, reporting “alignment 4, but alignment 16 is required” at line 226.

Ordinary aarch64 callers can therefore encounter undefined behavior through a safe parsing API. Use an unaligned write or establish alignment from the actual destination pointer. Retain the ordinary-input Miri regression.

### 2. Exhausting the public deserializer panics instead of returning an error

[src/serde.rs:255](/work/crate/src/serde.rs:255), [src/lib.rs:876](/work/crate/src/lib.rs:876).

After successfully deserializing `u64` from a `Deserializer` containing `1`, a second safe `u64::deserialize(&mut de)` call should return an exhaustion error. It instead panics with an out-of-bounds index in debug mode.

Executed evidence: `exhausted_deserializer_returns_error` failed with “the len is 1 but the index is 1.” Source inspection shows numeric deserialization calls unchecked `next_()`; release behavior was not executed.

Use checked tape access in safe Serde entry points and return `Eof` when exhausted.

## Medium

### 3. Long decimals produce different values depending on their input offset

[src/numberparse/correct.rs:168](/work/crate/src/numberparse/correct.rs:168).

With default features, `1.2345678901234567890123456789` parses correctly at offset zero. After 64 spaces, it becomes approximately `5.097733592125637e-10`, instead of `1.2345678901234567`.

Executed evidence: `float_value_is_independent_of_byte_offset` compared five decimal forms at 140 offsets against Rust’s `f64` parser and found substantial errors.

The code saves the starting digit’s byte value where it needs a starting position, incorrectly bypassing the slow conversion path. Ordinary documents can silently acquire wrong numeric values. Save the position and add offset-invariance regressions.

### 4. Extra tuple elements are accepted and can replace sibling values

[src/serde/de.rs:254](/work/crate/src/serde/de.rs:254), also array-form structs at line 322.

Deserializing `[[1,2,3],4]` as `((u64,u64),u64)` should reject the extra inner element. It returns `Ok(((1,2),3))`: the outer sibling comes from inside the inner array.

Executed evidence: three regressions also showed `[1,2,3]` accepted as a two-element tuple and as a two-field struct. `serde_json` rejected the tuple cases.

Validate container consumption after visitors return and enforce tuple lengths. A top-level trailing-data check alone would not fix nested value shifting.

### 5. Unknown-length empty containers serialize as incomplete JSON

[src/serde/se.rs:96](/work/crate/src/serde/se.rs:96), [src/serde/se/pp.rs:521](/work/crate/src/serde/se/pp.rs:521).

Executed evidence:

- Serializing an empty filtered iterator through Serde’s `collect_seq` returns `Ok("[")`.
- Pretty-serializing a struct containing only an empty flattened map returns `Ok("{")`.

Callers receive success while writing unreadable JSON. The serializers confuse “no elements written” with “closing delimiter already written.” Track those states separately and close containers started with an unknown length.

### 6. Pretty tuple-enum serialization omits the closing array

[src/serde/se/pp.rs:236](/work/crate/src/serde/se/pp.rs:236).

Pretty-serializing `Event::Point(1,2)` should produce an object containing `[1,2]`. The returned text ends with `}` while its array remains open.

Executed evidence: `pretty_tuple_variant_round_trip` failed when reading the result with `serde_json`; compact serialization of the same value passed.

Close the array before closing the surrounding enum object, including empty tuple variants.

### 7. Consuming DOM conversions reject numeric map keys

[src/serde/value/owned/de.rs:189](/work/crate/src/serde/value/owned/de.rs:189), [src/serde/value/borrowed/de.rs:202](/work/crate/src/serde/value/borrowed/de.rs:202).

A `BTreeMap<u32,String>` serialized into either DOM representation should deserialize back. Both consuming conversions reject key `"42"` with “expected u32,” while the corresponding reference conversions succeed.

Executed evidence: separate owned and borrowed map regressions demonstrated this discrepancy.

Consuming object access passes keys through the ordinary string-value deserializer. Use the existing map-key deserializer so ownership choice does not change accepted key types.

### 8. Tape accessors lose supported 128-bit integers

[src/value/tape/trait_impls.rs:48](/work/crate/src/value/tape/trait_impls.rs:48).

With `128bit` enabled, parsing `18446744073709551616` succeeds, but `tape.as_value().as_u128()` returns `None`.

Executed evidence: `wide_tape_accessors` failed on that value. Source inspection shows the tape implementation omits direct 128-bit accessors, leaving defaults that depend on narrower accessors.

Callers cannot retrieve a successfully parsed integer through its matching accessor. Delegate `as_i128` and `as_u128` to the underlying `StaticNode` and test signed and unsigned boundaries.

### 9. Enabling `128bit` makes valid integer-to-float deserialization fail

[src/serde.rs:331](/work/crate/src/serde.rs:331).

With `128bit` enabled, deserializing `18446744073709551616` into `f64` should produce its finite floating-point value. It returns `ExpectedFloat`.

Executed evidence: `wide_integer_as_float` failed. `parse_double` handles 64-bit integer nodes but omits 128-bit nodes.

Applications using floating-point fields can reject valid input after this feature is enabled. Add conversions for both wide integer variants; the shared path also serves `f32`.

### 10. Large exponents wrap before conversion

[src/numberparse/correct.rs:186](/work/crate/src/numberparse/correct.rs:186).

`1e4294967296` should follow the parser’s overflow policy and return an error. It returns `Ok(1.0)`.

Executed evidence: `large_exponent_does_not_wrap` reproduced the result. The accumulated exponent is narrowed from `i64` to `i32`, wrapping to zero.

Malformed or out-of-range numeric data can silently become an unrelated finite value. Check the conversion or use the existing slow parser before narrowing.

### 11. Large-integer parsing ignores invalid suffixes

[src/numberparse/correct.rs:190](/work/crate/src/numberparse/correct.rs:190), return path at line 239.

`1234567890123456789x` is invalid JSON and should be rejected. `to_owned_value` returns the integer `1234567890123456789`, discarding the suffix.

Executed evidence: `reject_large_integer_suffix` failed; the reference parser rejected the input.

The large-integer branch bypasses the normal token-terminator check. Validate the terminator before returning from slow numeric paths.

### 12. Invalid surrogate pairs are silently converted into unrelated characters

[src/stringparse.rs:77](/work/crate/src/stringparse.rs:77).

A high surrogate must be followed by a low surrogate. The code checks only the second value’s lower bound.

Executed evidence: `"\ud800\ue000"` deserialized successfully as `"𐐀"` rather than returning an invalid-Unicode error. Valid minimum and maximum surrogate-pair controls passed.

Callers can receive altered text from malformed Unicode escapes. Require the second value to be within `0xdc00..0xe000` before combining it.

### 13. Approximate number parsing has broken feature combinations

[src/numberparse/approx.rs:1](/work/crate/src/numberparse/approx.rs:1), [src/numberparse/approx.rs:339](/work/crate/src/numberparse/approx.rs:339).

Two isolated configurations fail even with warnings capped:

- `128bit,approx-number-parsing`: `static_cast_i128!` is not imported.
- `--no-default-features --features approx-number-parsing`: imports reference helpers disabled without `swar-number-parsing`.

Executed evidence: both `cargo check` commands failed with the corresponding compiler errors. These failures are independent of the nightly-only `hints` feature.

Import the wide cast under its feature gate and gate SWAR-only imports consistently. Add these combinations to feature checks.

## Low

### 14. Strict checks fail on the supplied compiler and host

[src/lib.rs:1](/work/crate/src/lib.rs:1), [src/impls/neon/stage1.rs:207](/work/crate/src/impls/neon/stage1.rs:207).

On Rust 1.99.1/aarch64, strict default Clippy fails on three unused-import diagnostics and `manual_is_multiple_of`. The crate’s warning-denial policy turns these into build errors.

Executed evidence: default `cargo clippy --locked --workspace --all-targets -- -D warnings` failed with four errors. Tests pass with the requested lint cap.

Remove obsolete imports and address compatible lint changes. CI specifies Rust 1.88; that toolchain was unavailable, so its result was not verified.

## Verification

Commands ran with `RUSTUP_AUTO_INSTALL=0`, on Rust/Cargo 1.99.1, aarch64 Linux.

From `/work/crate`:

- `RUSTFLAGS="--cap-lints warn" cargo test --locked` — passed: 464 tests, 3 ignored.
- `RUSTFLAGS="--cap-lints warn" cargo test --locked --no-default-features` — passed: 349 tests, 2 ignored.
- `RUSTFLAGS="--cap-lints warn" cargo test --locked --features "128bit,known-key,value-no-dup-keys,beef,arraybackend,ordered-float"` — passed: 493 tests, 4 ignored.
- `RUSTFLAGS="--cap-lints warn" cargo test --locked --features approx-number-parsing` — passed: 456 tests, 3 ignored.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — failed as described above.
- `RUSTFLAGS="--cap-lints warn" RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --document-private-items` — passed.
- `RUSTFLAGS="--cap-lints warn" cargo check --locked --features "128bit,approx-number-parsing"` — failed.
- `RUSTFLAGS="--cap-lints warn" cargo check --locked --no-default-features --features approx-number-parsing` — failed.
- `bash /work/[tooling]/scripts/[tooling] --audit /work/crate` — formatting passed; all-feature Clippy, documentation and tests failed. Besides the missing macro import, this configuration enables the documented nightly-only `hints` feature on stable.

The heuristic scan and a separate default-feature advisory Clippy pass were also executed. Advisory diagnostics included 126 unique unsafe/unsafe-impl locations without recognized safety comments, 13 types without `Debug`, and 69 `allow` locations without reason attributes. These mechanical counts are not additional confirmed behavior findings.

From `/work/tmp/probe`, using `RUSTFLAGS="--cap-lints warn"`:

- `cargo test --offline --test behavior -- --nocapture` — 2 passed, 6 failed.
- `cargo test --offline --features wide --test behavior -- --nocapture` — 2 passed, 8 failed.
- `cargo test --offline --test numbers -- --nocapture` — 1 passed, 1 failed.
- `cargo test --offline --test serde_edges -- --nocapture` — 1 passed, 5 failed.
- `cargo test --offline --features "dedup,known" --test controls -- --nocapture` — all 5 passed, covering string boundaries, buffer reuse, lazy mutation, duplicate-key handling, known keys and valid Unicode pairs.

For Miri, `/work/tmp/review` contains a scratch copy with only `#![deny(warnings)]` removed and one added integration-test file:

- `cargo +nightly-2026-10-01 miri test --locked --test review_miri ordinary_parse_and_serialize` — stopped on the alignment error.
- `cargo test --locked --test review_miri exhausted_deserializer_returns_error -- --nocapture` — failed with the exhaustion panic.

Full-suite Miri, Rust 1.88 checks, cross-target checks and a feature-powerset run were not performed. No security assessment, exploit investigation, fuzz campaign or resource-exhaustion testing was performed.

**Finding count: 14 — 2 High, 11 Medium, 1 Low.** Coverage was focused on documented parsing, DOM/tape access, Serde conversion and serialization, ordinary boundaries, and local build configurations. Last clock reading: **604.5 seconds active time, 10.07 of 45 minutes**.