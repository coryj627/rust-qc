The baseline passes with the supplied lint cap, but targeted regressions expose the following defects. `/work/crate` remains unchanged.

1. **Long floats produce different values depending on their position.** [src/numberparse/correct.rs:168](/work/crate/src/numberparse/correct.rs:168)  
   Parsing should be independent of leading whitespace. `12345678901234567890.0` parses correctly at offset zero, but after 60 spaces returns `1.2776324570088369e18` instead of `1.2345678901234567e19`. The digit-count calculation uses a character’s ASCII value as a position, allowing an overflowed significand through the fast path. Confirmed by `long_float_independent_of_offset`, with default features and `128bit`.

2. **Tape container accessors truncate contents and panic on empty containers.** [src/value/tape/trait_impls.rs:425](/work/crate/src/value/tape/trait_impls.rs:425)  
   `get_array`, `get_object`, and their `try_` variants should return complete containers. Their slices omit one node because `count` excludes the header. For `[1,2]`, the returned array reports length two but iterates once and returns `None` for index one. Empty arrays and objects panic when querying their length. Confirmed for all four accessors by `boundary_observations`; the ordinary `get(...).as_array()` route works correctly.

3. **Ordinary aarch64 parsing performs a misaligned SIMD store.** [src/impls/neon/stage1.rs:207](/work/crate/src/impls/neon/stage1.rs:207), [line 226](/work/crate/src/impls/neon/stage1.rs:226)  
   The code selects aligned `ptr::write` using the element offset modulo four, although `Vec<u32>` does not guarantee 16-byte alignment. Miri stopped during a valid JSON round trip: the pointer had alignment eight, while `int32x4_t` required sixteen. This violates Rust’s alignment requirements in the normal NEON parsing path. Native tests passed; no native crash is claimed.

4. **Extra tuple elements can become the following field’s value.** [src/serde/de.rs:254](/work/crate/src/serde/de.rs:254)  
   Deserializing `[[1,2,3],4]` into `((u8,u8),u8)` should reject the oversized inner tuple. Instead, it returns `((1,2),3)`, silently ignoring `4`. Container visitors are not checked for complete consumption. Confirmed by `tuple_extra_elements_rejected`; `serde_json` rejects the same input.

5. **Consuming DOM deserialization rejects supported numeric map keys.** [src/serde/value/owned/de.rs:189](/work/crate/src/serde/value/owned/de.rs:189), [src/serde/value/borrowed/de.rs:202](/work/crate/src/serde/value/borrowed/de.rs:202)  
   Converting `{"1":2}` into `BTreeMap<u32,u32>` should behave consistently across DOM conversion APIs. Conversion by reference succeeds, but consuming either an owned or borrowed DOM fails with “invalid type: string … expected u32.” These paths deserialize keys as ordinary string values instead of using the map-key adapter. Confirmed by `consume_dom_numeric_map_keys` and `boundary_observations`.

6. **Empty sequences without length hints serialize as incomplete JSON.** [src/serde/se.rs:96](/work/crate/src/serde/se.rs:96), [src/serde/se/pp.rs:134](/work/crate/src/serde/se/pp.rs:134), [line 521](/work/crate/src/serde/se/pp.rs:521)  
   A valid `serialize_seq(None)` followed by `end()` should produce `[]`; compact and pretty serializers both return `"["` successfully. An empty `serialize_map(None)` similarly produces `"{"` with the pretty serializer. The `first` flag incorrectly doubles as an indication that the closing delimiter was already written. Confirmed by `unknown_length_empty_sequence` and `boundary_observations`.

7. **Nonfinite floats serialize into invented decimal values.** [src/serde/se.rs:641](/work/crate/src/serde/se.rs:641), [src/serde/se/pp.rs:696](/work/crate/src/serde/se/pp.rs:696)  
   Serde-compatible serialization should represent these values as `null`, or explicitly reject them. Instead, NaN becomes `"2.696539702293474e308"` and positive infinity becomes `"1.797693134862316e308"`, with successful results. Both serializers pass nonfinite values to a finite-number formatter. DOM `encode()` also exhibits this behavior. Confirmed by `nonfinite_float_serialization` and `boundary_observations`.

8. **Tape scalar access loses 128-bit values.** [src/value/tape/trait_impls.rs:48](/work/crate/src/value/tape/trait_impls.rs:48)  
   With `128bit`, representable integers should be available through `as_i128` and `as_u128`. A tape containing `18446744073709551616` returns `None` from `as_u128`, while the equivalent DOM returns the correct value. The implementation inherits defaults that narrow through 64-bit accessors. Confirmed by `tape_128_scalar_access` and `tape_large_128_scalar_access`. Source inspection also found that existing conversion tests in [src/value/tape.rs:197](/work/crate/src/value/tape.rs:197) exercise a `StaticNode` alias rather than tape values.

9. **Serde float deserialization rejects parsed 128-bit integers.** [src/serde.rs:331](/work/crate/src/serde.rs:331)  
   With `128bit`, `from_slice::<f64>` should accept the exactly representable number `18446744073709551616`. It instead returns `ExpectedFloat`, because `parse_double` handles only 64-bit integer nodes. Ordinary floating-point fields therefore reject otherwise supported JSON numbers. Confirmed by `deserialize_large_integer_as_float`.

10. **Malformed surrogate escapes silently change string contents.** [src/stringparse.rs:61](/work/crate/src/stringparse.rs:61), [line 77](/work/crate/src/stringparse.rs:77)  
    Consistent with the existing invalid-surrogate test and Serde behavior, invalid pairs should return an error. `"\ud800"` instead becomes a NUL string, and `"\ud800\ue000"` becomes `"𐐀"`. The first path returns code point zero; the second fails to check the upper bound of the low-surrogate range. Confirmed by `unpaired_high_surrogate_rejected` and `boundary_observations`. The existing regression covers only a lone low surrogate.

11. **Large exponents wrap into small exponents.** [src/numberparse/correct.rs:186](/work/crate/src/numberparse/correct.rs:186)  
    Exponent narrowing should preserve the value or reject an unsupported range. Instead, `1e4294967296` becomes `1.0`, and `1e-4294967296` also becomes `1.0`, rather than an overflow error and underflow to zero respectively. Casting the accumulated exponent to `i32` wraps it. Confirmed by `number_exponent_boundaries` and `boundary_observations`.

12. **Large integers bypass trailing-character validation.** [src/numberparse/correct.rs:190](/work/crate/src/numberparse/correct.rs:190), [line 239](/work/crate/src/numberparse/correct.rs:239)  
    Invalid numeric tokens should return an error. Both `1234567890123456789x` and `[1234567890123456789x]` are accepted, silently discarding the suffix. The large-integer path returns before the delimiter validation used for smaller integers. Confirmed by `large_integer_trailing_letters_rejected` and `boundary_observations`, including with `128bit`.

13. **A no-op lazy-value upgrade changes equality.** [src/value/lazy.rs:43](/work/crate/src/value/lazy.rs:43)  
    The documented upgrade preserves the JSON value, so callers using equality for change detection should see unchanged content. Two equal clones become unequal after merely calling `as_object_mut()` on one, without modifying any entry. Derived enum equality compares storage variants. Confirmed by `lazy_value_equality_survives_upgrade`.

14. **Malformed literals are classified as data-shape errors.** [src/error.rs:234](/work/crate/src/error.rs:234)  
    Invalid JSON such as `falsex` should be classified as a syntax error. It produces `ExpectedFalse`, with `is_syntax() == false` and `is_data() == true`. Callers relying on the documented classification can misroute syntax failures. Confirmed by `invalid_literal_error_classification` and `literal_error_observations`.

15. **Identical structured errors compare unequal.** [src/error.rs:149](/work/crate/src/error.rs:149)  
    Two `Unexpected` errors with identical expected and actual types should compare equal. They fall through to `false`; even an error compared with itself is unequal. This breaks error comparisons and assertions. Confirmed by `error_type_equality`; source inspection identifies the omitted match arm.

16. **The `big-int-as-float` regression suite does not compile.** [src/numberparse/correct.rs:584](/work/crate/src/numberparse/correct.rs:584)  
    The feature’s tests should compile. `huge_int` compares `OwnedValue` with `StaticNode`, producing E0277. `cargo check` for the feature succeeds, so this is a test-target failure rather than a demonstrated library build failure.

All behavioral reproductions are retained in [review_regressions.rs](/work/tmp/review/tests/review_regressions.rs). Their failed assertions express the expected behavior; `boundary_observations` records additional variants and catches the reported container panics.

Checks used Rust/Cargo 1.99.1 on aarch64 Linux with `RUSTUP_AUTO_INSTALL=0`. Unless noted otherwise, the following commands ran from `/work/crate` with `RUSTFLAGS="--cap-lints warn"`:

| Command | Outcome |
|---|---|
| `cargo test --locked` | Passed: 313 unit, 130 integration, 21 doctests; three ignored |
| `cargo test --locked --no-default-features` | Passed |
| `cargo test --locked --no-default-features --features serde_impl` | Passed |
| `cargo test --locked --features 128bit` | Passed |
| `cargo test --locked --features approx-number-parsing` | Passed |
| `cargo test --locked --features known-key,beef,ordered-float,value-no-dup-keys,arraybackend` | Passed |
| `cargo test --locked --features big-int-as-float` | Failed compiling the test target: E0277 |
| `cargo check --locked --features big-int-as-float` | Passed |
| `cargo clippy --locked --all-targets` | Exit zero under the lint cap; warnings remain |
| `cargo fmt --all -- --check` | Passed |
| `RUSTDOCFLAGS="--cap-lints warn" cargo doc --locked --no-deps` | Passed |
| `cargo test --locked --lib -- --ignored` | One passed; the intentionally ignored Serde float-comparison test failed |

The ignored float comparison differed by one rounding step between parsers; that failure alone is not counted as a defect.

Uncapped `cargo check --locked` failed on the pre-existing unused-import warnings identified in the task: [src/numberparse/correct.rs:12](/work/crate/src/numberparse/correct.rs:12), [src/lib.rs:47](/work/crate/src/lib.rs:47), and [src/stage2.rs:3](/work/crate/src/stage2.rs:3). This known compiler compatibility failure is not included in the finding count.

Scratch regression commands, run from `/work/tmp/review`:

```sh
RUSTUP_AUTO_INSTALL=0 RUSTFLAGS="--cap-lints warn" cargo test --locked --test review_regressions -- --test-threads=1 --nocapture
RUSTUP_AUTO_INSTALL=0 RUSTFLAGS="--cap-lints warn" cargo test --locked --test review_regressions --features 128bit -- --test-threads=1 --nocapture
```

Outcomes: respectively **4 passed / 12 failed** and **3 passed / 15 failed**.

Miri ran from `/work/tmp/miri-review`, with only `#![deny(warnings)]` removed from its copied library source:

```sh
RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true cargo +nightly-2026-10-01 miri test --locked --test review_regressions smoke_valid_roundtrip -- --exact
```

It exited one at the alignment violation in finding 3; the round trip did not complete under Miri.

Contributor coordination directions in [src/macros.rs:209](/work/crate/src/macros.rs:209) were treated as repository data, not instructions.

**Finding count: 16.** Coverage limits: representative documented feature configurations, native aarch64 execution, and one Miri smoke test; no exhaustive feature combinations, other architectures, MSRV validation, full Miri suite, or dedicated security assessment. **Last clock reading: 8.250033333333333 minutes used of 45.0.**