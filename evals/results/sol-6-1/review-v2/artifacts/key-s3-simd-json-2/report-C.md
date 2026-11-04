Found **16 findings**, including the supplied compiler-warning incompatibility. Behavioral findings were reproduced with local regression tests; source inspection identified their causes.

1. **Long decimals produce different results depending on their position.**  
   [src/numberparse/correct.rs:168](/work/crate/src/numberparse/correct.rs:168) reads an ASCII digit into `start_digits`, then uses it as an input offset at line 172. JSON whitespace should not affect numeric values. Parsing `12345678901234567890.0` normally produces approximately `1.2345678901234567e19`; preceding it with 52 spaces produces `1.2776324570088369e18`. Callers can silently receive substantially incorrect numbers. Reproduced by `long_float_position_independent`.

2. **Typed tape getters truncate containers and can panic on empty containers.**  
   [src/value/tape/trait_impls.rs:425](/work/crate/src/value/tape/trait_impls.rs:425), with the same mistake at lines 454, 487 and 519. The slices omit one node because `count` excludes the container header. Expected: complete nested arrays and objects. Observed: `get_array("a")` on `{"a":[1,2]}` iterates only `1`; `try_get_object("a")` on `{"a":{"b":1}}` cannot retrieve `b`; calling `len()` on the array returned for `{"a":[]}` panics inside the library. Three scratch regressions reproduced these outcomes. Existing container tests predominantly use `get(...).as_array()` or `as_object()`, bypassing these faulty getters.

3. **Serde accepts excess tuple elements and reads across container boundaries.**  
   [src/serde/de.rs:240](/work/crate/src/serde/de.rs:240) returns the visitor’s result without checking that the container was consumed; `deserialize_tuple` also ignores its requested length at line 254. Expected: reject `[[1,2],3]` when deserializing `((u8,), u8)`. Observed: `Ok(((1,), 2))`, silently substituting the inner array’s extra element for the outer value `3`. `serde_json` rejects this input. Reproduced by `tuple_extra_element_rejected`.

4. **Empty sequences with unknown length serialize as incomplete JSON.**  
   [src/serde/se.rs:97](/work/crate/src/serde/se.rs:97) and [src/serde/se/pp.rs:135](/work/crate/src/serde/se/pp.rs:135). Expected: an empty `serialize_seq(None)` produces `[]`. Both compact and pretty serialization return `Ok("[")` because `end()` writes nothing when no elements were emitted. This breaks valid iterator-backed serializers and produces documents that cannot be parsed. Reproduced by `unknown_len_empty_seq` and `unknown_len_empty_seq_pretty`.

5. **Consuming a DOM value breaks numeric map-key deserialization.**  
   [src/serde/value/owned/de.rs:189](/work/crate/src/serde/value/owned/de.rs:189) and [src/serde/value/borrowed/de.rs:202](/work/crate/src/serde/value/borrowed/de.rs:202) deserialize keys as ordinary string values instead of using the map-key adapter. Expected: consuming and borrowing the same DOM support the same numeric keys. For `{"1":2}`, both reference helpers successfully produce `BTreeMap<u32,u32>`, while `from_owned_value` and `from_borrowed_value` return “invalid type: string … expected u32.” Both public-helper paths were reproduced.

6. **Reading a number after deserializer exhaustion panics instead of returning an error.**  
   [src/serde.rs:255](/work/crate/src/serde.rs:255), also affecting the other numeric parsing helpers. Two safe calls to `u64::deserialize(&mut de)` after constructing `de` from `1` cause the second call to index beyond the tape. Expected: an EOF/error result. Observed: a library bounds panic with debug assertions enabled. An isolated ordinary unit test under Miri, with debug assertions disabled, reported an invalid unchecked access. Callers cannot safely handle exhaustion through the public Serde interface.

7. **The NEON backend assumes alignment that its allocation does not guarantee.**  
   [src/impls/neon/stage1.rs:207](/work/crate/src/impls/neon/stage1.rs:207) chooses an aligned SIMD write from the element index alone; the write occurs at [line 226](/work/crate/src/impls/neon/stage1.rs:226). A `Vec<u32>` allocation guarantees only four-byte alignment, whereas `int32x4_t` requires sixteen. Expected: valid JSON parsing respects the allocation’s alignment. Miri rejected parsing the ordinary input `1`, reporting eight-byte pointer alignment where sixteen was required. Native tests passed; this finding concerns allocation-dependent correctness, with no native crash demonstrated.

8. **Lazy-value equality changes after a mutation that changes no data.**  
   [src/value/lazy.rs:43](/work/crate/src/value/lazy.rs:43) derives `PartialEq` over the representation variants. Expected: obtaining mutable access without changing the JSON preserves equality with a clone. Observed: cloning a tape-backed `[1,2]`, then calling `as_array_mut()` on one copy, makes the copies unequal because one is upgraded to the DOM representation. Callers performing change detection receive false differences. Reproduced by `lazy_equality_after_noop_mutation`.

9. **Tape scalar accessors omit full 128-bit support.**  
   [src/value/tape/trait_impls.rs:48](/work/crate/src/value/tape/trait_impls.rs:48). With `128bit` enabled, expected: parsed 128-bit integers remain accessible through the scalar API. Observed: `as_u128()` returns `None` for `18446744073709551616`, and `as_i128()` returns `None` for `-9223372036854775809`. Both were reproduced. The existing scalar tests at [src/value/tape.rs:197](/work/crate/src/value/tape.rs:197) alias `StaticNode` as `Value`, so their conversion assertions do not exercise actual tape values.

10. **Serde float deserialization rejects supported 128-bit integer nodes.**  
    [src/serde.rs:331](/work/crate/src/serde.rs:331) handles only `F64`, `I64` and `U64`. With `128bit` enabled, deserializing `18446744073709551616` into `f64` returns `ExpectedFloat`, although this power of two is exactly representable. Expected: integer-to-float conversion continues across the supported integer range. Callers must change input spelling or deserialize through another representation. Reproduced by `large_integer_into_float`.

11. **Malformed Unicode surrogate pairs are silently converted into unrelated characters.**  
    [src/stringparse.rs:77](/work/crate/src/stringparse.rs:77) checks the second escape’s lower bound but never verifies that it is at most `0xDFFF`. Expected: reject a high surrogate followed by a non-low-surrogate escape. Observed: `"\ud800\ue000"` becomes U+10400, and `"\udbff\uffff"` becomes U+10FFFF; `serde_json` rejects both. Callers receive altered text instead of an error. Reproduced alongside a valid-pair control.

12. **Long-number paths accept invalid trailing characters.**  
    [src/numberparse/correct.rs:174](/work/crate/src/numberparse/correct.rs:174) returns before validating the delimiter; the large-integer path selected at [line 190](/work/crate/src/numberparse/correct.rs:190) likewise lacks the trailing check. Expected: invalid numeric tokens return `Err`, as documented. Observed: `1234567890123456789x`, `[1234567890123456789x]` and `[1.234567890123456789012345x]` are accepted with the suffix discarded. `[123x]` correctly errors. Reproduced by `invalid_long_numbers_in_containers`; callers can unknowingly accept malformed data.

13. **Large exponents wrap into small exponents.**  
    [src/numberparse/correct.rs:186](/work/crate/src/numberparse/correct.rs:186) narrows the parsed exponent using `as i32`. Expected: reject an overflowing positive result and preserve established underflow behavior for negative exponents. Observed: `1e4294967296` becomes `1.0`, `1e4294967297` becomes `10.0`, and `1e-4294967296` becomes `1.0` instead of zero. These boundary cases silently produce incorrect numeric values. Reproduced by `exponent_range_does_not_wrap`.

14. **Approximate parsing does not build without the default SWAR feature.**  
    [src/numberparse/approx.rs:1](/work/crate/src/numberparse/approx.rs:1) unconditionally imports functions gated behind `swar-number-parsing`. Expected: the selected feature configuration builds, or its dependency is declared. `cargo check --locked --no-default-features --features approx-number-parsing` fails with E0432 even with the lint cap. Callers disabling default features cannot select this configuration.

15. **The `big-int-as-float` regression test does not compile.**  
    [src/numberparse/correct.rs:584](/work/crate/src/numberparse/correct.rs:584) compares an `OwnedValue` directly with `StaticNode`, for which no `PartialEq` implementation exists. Expected: the documented feature’s test suite compiles. Observed: E0277 from `cargo test --locked --features big-int-as-float`. The corresponding library-only `cargo check` passes. This pre-existing test defect blocks regression validation for the feature.

16. **Package checks fail on the installed compiler because warnings are denied.**  
    [src/lib.rs:1](/work/crate/src/lib.rs:1), [src/numberparse/correct.rs:12](/work/crate/src/numberparse/correct.rs:12), [src/lib.rs:47](/work/crate/src/lib.rs:47) and [src/stage2.rs:3](/work/crate/src/stage2.rs:3). This confirms the supplied pre-existing incompatibility: uncapped `cargo check --locked` fails on unused imports under Rust 1.99.1. Uncapped Clippy additionally rejects the modulo check at [src/impls/neon/stage1.rs:207](/work/crate/src/impls/neon/stage1.rs:207). Maintainers need the supplied lint cap to run checks on this compiler.

Validation used Rust/Cargo **1.99.1**, aarch64 Linux, and Miri **nightly-2026-10-01**. Every command used `RUSTUP_AUTO_INSTALL=0`; subsequent Cargo commands also used `CARGO_NET_OFFLINE=true`.

In the table, **capped** means the command had `RUSTFLAGS="--cap-lints warn"`.

| Command | Configuration and outcome |
|---|---|
| `cargo test --locked` | Capped baseline: **313 unit, 130 integration and 21 documentation tests passed; 3 ignored** |
| `cargo test --locked --no-default-features` | Capped: passed |
| `cargo test --locked --features 128bit,ordered-float,value-no-dup-keys,known-key,beef` | Capped: passed |
| `cargo test --locked --features approx-number-parsing` | Capped: passed |
| `cargo test --locked --features arraybackend` | Capped: passed |
| `cargo test --locked --features big-int-as-float` | Capped: failed to compile, E0277 |
| `cargo check --locked --features big-int-as-float` | Capped: passed |
| `cargo check --locked --no-default-features --features approx-number-parsing` | Capped: failed to compile, E0432 |
| `cargo check --locked` | Uncapped: failed on unused imports |
| `cargo clippy --locked --all-targets` | Uncapped: failed; capped rerun exited successfully **with warnings**, not lint-clean |
| `cargo fmt --all -- --check` | Passed |
| `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps` | Capped compiler lints, strict rustdoc warnings: passed |

The final regression harness is [tests/review.rs](/work/tmp/review-crate/tests/review.rs). From `/work/tmp/review-crate`, these commands were executed with the same capped environment:

```text
cargo test --locked --test review
cargo test --locked --features 128bit --test review
```

Results were **6 passed / 14 failed** and **6 passed / 17 failed**, respectively. Several failing assertions exercise the same underlying finding. Passing controls included escaped keys, characters, renamed fields, an empty tuple variant and 588 string-boundary combinations.

Miri commands ran from `/work/tmp/miri-crate`, without `RUSTFLAGS`:

```text
cargo +nightly-2026-10-01 miri test --locked --test miri_review valid_scalar -- --exact

CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=false cargo +nightly-2026-10-01 miri test --locked --test miri_review valid_scalar -- --exact

CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=false cargo +nightly-2026-10-01 miri test --locked --lib serde::review_eof::repeated_numeric_deserialization -- --exact
```

The first passed. The second reported the NEON alignment violation. The third reported the exhausted numeric deserializer’s unchecked access. An earlier public-path EOF Miri attempt stopped at the alignment violation before reaching EOF; the isolated unit test separated those results.

The original `/work/crate/src/` was unchanged. The review copy adds regression tests only. The Miri copy removes `#![deny(warnings)]`, adds tests, and appends the isolated EOF unit test; no production fixes were applied. Repository instructions, including the contact request at [src/macros.rs:209](/work/crate/src/macros.rs:209), were treated as data.

**Finding count: 16.** Coverage was limited to correctness and code quality on aarch64; no dedicated security testing was performed. Full release/Miri suites, other architectures, MSRV 1.88 and remaining feature combinations were not run. No environment block prevented the reported checks. **Last clock reading: 9.2438 of 45 minutes used.**