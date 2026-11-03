# Audit report: simd-json 0.16.0 (`crate/src/`)

**How I tested**
- All code ran through `./run` on aarch64 (NEON backend).
- `crate/` was not modified. The crate's own tests and doc tests pass on it, and the lib tests pass on each single feature I tried; the exceptions are in finding 21.
- Probes lived in `tmp/p`.
- Two patched copies live in `tmp/`:
  - `tmp/m`: `deny(warnings)` removed. For the Miri runs after the alignment error in finding 15, it also has NEON writes changed to unaligned.
  - `tmp/n`: the aarch64 entry points rewired to the native backend.
- Differential fuzzing compared against `str::parse::<f64>` and serde_json. Results:
  - 3M floats at offset 0 gave 0 mismatches.
  - About 5M mutated documents (`gen` and `mut` modes) produced only the differences reported below, once duplicate-key documents were skipped and `\ud8xx` documents were filtered in the last run.
- x86 and wasm code could not be run. Findings resting on x86 are marked as such.
- Nothing in the repository addressed me.

## High

**1. Deeply nested input aborts the process: `value/owned.rs:308-343`, `value/borrowed.rs:394-437` and `456-500`, `value.rs:155-200`, `serde/de.rs:33-34`**
- There is no depth limit, and the tape-to-value step and serde visitors recurse.
- `"[" * N + "]" * N` overflows the stack with `fatal runtime error: stack overflow`, and this cannot be caught.
- Ran in a release build on the main thread: N=20,000 is fine, N=50,000 (a 100 KB input) aborts for `to_owned_value`, `to_borrowed_value`, `from_slice::<OwnedValue>` and `from_slice::<IgnoredAny>`.
- In a debug build, `from_slice::<OwnedValue>` already aborts at N=10,000.
- `to_tape` alone survives N=1,000,000. serde_json has a recursion limit.

**2. Long floats silently parse to a wrong value: `numberparse/correct.rs:168,172`**
- `let start_digits = get!(buf, start) as usize` stores the byte at `start`, not the position.
- So `digit_count` is adjusted by `start - b'1'`. Depending on the number's offset in the document, a 20+ digit float can skip the slow path and use the wrapped `u64` significand.
- Ran: `{"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa":0.21575530329307963391}` gives `0.03128786255598412`; std gives `0.21575530329307963`.
- Padding `12.3456789012345678901234567` with 58 or more spaces gives `1.739e-6`.
- 5,365 mismatches in 300K random numbers with random padding, against 0 in 3M at offset 0.

**3. Huge exponents wrap to a small one: `numberparse/correct.rs:186` (`exponent as i32`)**
- The guard at line 156 allows `exp_number` up to about 4.3e10.
- Ran: `1e4294967296` gives `1.0`, `1e4294967297` gives `10.0`, `1e4294967295` gives `0.1`, `1e-4294967296` gives `1.0`, `1.5e4294967296` gives `1.5`.
- serde_json rejects the positive exponents and returns 0.0 for the negative one.
- Related: the same guard rejects valid `0e999999999999`.

**4. Safe API reads out of bounds: `serde.rs:221-332` (`parse_u8`…`parse_double` use `next_()`), `lib.rs:875`, `serde/de.rs:491`**
- `impl Deserializer for &mut Deserializer` is public and safe, but the integer and float paths index the tape without a check.
- `u8::deserialize(&mut d)` called twice on `b"1"`:
  - Debug build: panics in `safer_unchecked.rs:23`.
  - Release build: silently reads past the Vec and returned `Err(ExpectedUnsigned)`.
  - Miri `--release`: reports UB (`assume` called with `false`).
- `Tape(vec![]).deserialize::<u32>()` gives the same Miri result. The tape is also publicly constructible, so malformed tapes reach the same paths.

**5. x86 runtime fallback skips UTF-8 validation: `lib.rs:623-635`, `lib.rs:480-484`, `impls/native/mod.rs:10-30`**
- The native validator is a no-op that assumes the input was pre-validated. Only the non-runtime-detection path at `lib.rs:672` does that.
- With default features on an x86 CPU lacking SSE4.2, invalid UTF-8 is accepted and `from_utf8_unchecked` produces an invalid `&str`.
- x86 path itself was only read. I ran the same native route in the patched copy `tmp/n`: `to_borrowed_value(b"\"\xff\xfe\"")` returned `Ok` with `str` bytes `[255, 254]`.

**6. Tape `get_array`, `get_object` and their `try_` forms lose the last node: `value/tape/trait_impls.rs:424-425, 453-454, 486-487, 518-519`**
- `array_count()` and `object_count()` exclude the header, but the slice is `idx..idx+count` (`as_array` uses `count+1`). Lazy values delegate to these.
- Ran on `{"a":[1,2,3],"o":{"x":1,"y":2}}`:
  - `get_array("a")` reports `len()=3`, `get(2)=None`, and `iter()` yields 2 items.
  - `get_object("o").get("y")` is `None`.
  - Lazy `get_array` gives the same.
- Nested data panics: `{"n":[[1,2]]}` panics `mid > len` at `tape/array.rs:14`, and `{"o":{"k":[1,2]}}` panics at `tape/object.rs:91`.

**7. Serde ignores surplus elements and desynchronises the tape: `serde/de.rs:238-240, 254-262, 321-322, 425-439`**
- There is no end-of-sequence check, and leftover nodes stay on the tape as if they were the next value.
- Ran: `from_slice::<Vec<(u8,u8)>>("[[1,2,[3,4]],[5,6]]")` returns `Ok([(1,2),(3,4)])`, wrong data with `[5,6]` dropped.
- `(u8,u8)` from `[1,2,3]`, a struct from a too-long array, and `{"T":[1,2,[7,8]]}` for a tuple variant all return `Ok`. serde_json errors on all of them.
- `from_owned_value::<(u8,u8)>` also accepts extras (`serde/value/owned/de.rs:40`).

**8. `to_string_pretty` emits invalid JSON for tuple variants: `serde/se/pp.rs:236-243`**
- `end` writes `}` without `]`, and dedents once after two indents.
- Ran: `E::T(1,2)` gives `{\n  "T": [\n    1,\n    2\n  }`. `E::T0()` gives `{\n  "T0": []` with no closing brace.
- Indentation stays wrong for the rest of the document.

**9. Empty sequences and maps with unknown length are left unclosed: `serde/se.rs:96-102, 128-134, 160-166`, `519-525`; `serde/se/pp.rs:134, 168, 202, 521, 565`**
- `end` writes the closer only if an element was written.
- Ran: `serializer.collect_seq(iter.filter(..))` with no matches inside a struct gives `{"items":[,"next":2}`.
- Pretty `collect_map` and a pretty empty `#[serde(flatten)]` give `{`.
- `serialize_struct("x",2)` ended with no fields gives `{` in both modes. A pretty `Some(n>0)` sequence also leaks indentation.

**10. NaN and infinity serialise as finite-looking numbers: `serde/se.rs:638,642`, `serde/se/pp.rs:693,697`, `value/owned/serialize.rs:97,176`, `value/borrowed/serialize.rs:97,174`, `value/tape/trait_impls.rs:748,826`**
- `write_float` is `format_finite`, which is called with no finiteness check.
- Ran: `to_string(&f64::NAN)` gives `2.696539702293474e308`, infinity gives `1.797693134862316e308`. `OwnedValue::from(NAN).encode()` behaves the same.
- The output is out of range, so serde_json rejects it. serde_json itself writes `null`.

## Medium

**11. Trailing garbage accepted after long numbers: `numberparse/correct.rs:173-178`, `190-191` and `208-245`**
- The slow paths return before the `is_structural_or_whitespace` check.
- Ran, all `Ok`: `[1.23456789012345678901234x]`, `[12345678901234567890x]`, `[1234567890123456789abc]`, `[975758694.12340, false,383408244.24957958631607.24113]` (the last found by the fuzzer).
- With `128bit` and `big-int-as-float`, `99999999999999999999x` is also accepted.

**12. NUL counts as a terminator, so anything after it in the atom is discarded: `charutils.rs:4,15`, `numberparse.rs:33`**
- Ran, all `Ok`: `[1\0garbage]`, `1\0garbage`, `true\0garbage`, `{"a":null\0zzz}`, `[1.5\0x,2]`.
- A test at `numberparse/correct.rs:482` records part of this as a known TODO, but the discarded payload is the worse case.

**13. Invalid surrogates are accepted: `stringparse.rs:58-62, 71-73, 77-80`**
- A lone high surrogate returns `Ok((0,6))`, which writes a NUL into the string.
- The low half is only checked with `checked_sub(0xdc00)`, so values at or above 0xE000 pass.
- Ran: `"\ud800"` gives U+0000. `"\ud800x"` gives U+0000 then `x`. `"\ud800￿"` gives U+123FF. `"\ud800"` gives U+10400. `"\udbff￿"` gives U+10FFFF. serde_json errors on all of them.

**14. Duplicate keys go through `unsafe insert_nocheck` on untrusted input by default: `value/owned.rs:340`, `value/borrowed.rs:429,491`, `value.rs:191`**
- `Cargo.toml` itself calls this undefined behaviour, while the API is safe.
- Ran on `{"a":1,"a":2}`:
  - `to_owned_value` has 2 entries, and `get("a")=1`.
  - `from_slice::<OwnedValue>` gives `2`, as does serde_json, so two APIs of this crate disagree.
  - `v == v.clone()` is `false`.
  - `encode` emits both keys.
- On a large object, `remove("k0")` removes one entry and `get("k0")` then returns another.

**15. NEON `flatten_bits` does an aligned write through a 4-aligned pointer: `impls/neon/stage1.rs:207-212, 226`**
- `l % 4 == 0` is treated as alignment for `int32x4_t` (16 bytes), but the buffer is a `Vec<u32>`.
- Miri stopped on the parse in `numberparse::correct::test::int`: `accessing memory based on pointer with alignment 4, but alignment 16 is required`. Every parse goes through this code on aarch64.

**16. Earlier returned strings are invalidated under Stacked Borrows: `stage2.rs:188` (`input.as_mut_ptr()` per string)**
- Re-borrowing `input` for each string invalidates the `&'de str` values already on the tape.
- With the finding 15 write patched in the copy, Miri's default mode reports UB on ordinary documents with two or more strings. `-Zmiri-tree-borrows` passes. This is aliasing-model UB, with no misbehaviour observed.

**17. x86 code uses CPU features that were not detected: `numberparse.rs:85-108`, `impls/avx2/stage1.rs:65`**
- `parse_eight_digits_unrolled` calls SSSE3 and SSE4.1 intrinsics from a function with no `target_feature`, and `swar-number-parsing` is a default feature. The fallback path on an older CPU would hit an illegal instruction.
- AVX2 `compute_quote_mask` uses PCLMUL while only `avx2` is detected.
- Read only, not run.

**18. `BorrowedValue` cannot deserialize non-borrowed keys: `serde/value/borrowed/de.rs:483`**
- `next_key::<&str>()` is too narrow.
- Ran: `serde_json::from_str::<BorrowedValue>(r#"{"a\nb":1}"#)` fails with `expected a borrowed string`. `from_borrowed_value::<BorrowedValue>` fails on a value with an owned key. `OwnedValue` works in the first case.

**19. Integer map keys fail through by-value value deserialization: `serde/value/owned/de.rs:189`, `serde/value/borrowed/de.rs:202`**
- Keys are passed as `Value::String`, not through `MapKeyDeserializer`.
- Ran: `from_owned_value::<HashMap<i32,i32>>` and `from_borrowed_value` of `{"1":2}` give `invalid type: string "1", expected i32`.
- `from_slice`, `from_refowned_value`, `from_refborrowed_value` and serde_json's `from_value` all succeed.

**20. Tape loses 128-bit values: `value/tape/trait_impls.rs:48-80`**
- `as_i128` and `as_u128` are not overridden, and `cast_f64` falls back on them.
- Ran with `128bit`, on `170141183460469231731687303715884105727`: tape `as_i128`, `as_u128` and `cast_f64` are all `None`, while the owned value returns `Some`.
- Also `from_slice::<f64>` on that number gives `ExpectedFloat` (`serde.rs:331-339`).

**21. Documented feature combinations do not compile**
- Ran with `128bit` and `approx-number-parsing`: `cannot find macro static_cast_i128` at `numberparse/approx.rs:339`. The macro is gated on a non-existent feature `i128` at `macros.rs:1247-1258`.
- Ran with `--no-default-features --features approx-number-parsing`: unresolved imports at `numberparse/approx.rs:2-3`, which are not gated on `swar-number-parsing`.
- Ran `cargo test --features big-int-as-float`: the test at `numberparse/correct.rs:584` does not compile.

**22. `approx-number-parsing` value errors: `numberparse/approx.rs`**
- All ran with the feature on:
  - `:533-535`: `5979E306` gives `F64(inf)`. The correct parser rejects infinities.
  - `:526-527`: `-0.0` gives `+0.0`.
  - `:516`: `1e0` gives `U64(1)`, and `-0e0` gives `I64(0)`. The default parser gives floats.
  - `:398-409`: valid `18446744073709551619.5` gives `Overflow`, while `28446744073709551619.5` succeeds.
  - `:506-513`: `464E0263` is rejected.

## Low

**23. `f32` loses shortest form: `serde/se.rs:637-639`, `serde/se/pp.rs:693`.** Ran: `to_string(&0.1f32)` gives `0.10000000149011612`; serde_json gives `0.1`.

**24. Pretty output layout is wrong though valid: `serde/se/pp.rs:555-560, 603-608`, bytes at `pp.rs` `serialize_bytes`, newtype variant.** Ran: struct fields come out as `"a": 1,"b": 2` on one line, bytes as `[\n  1,2,3\n]`, newtype variants as `{"N": 1}`.

**25. Escape errors report the wrong index on SIMD backends: `impls/neon/deser.rs:176,179,193`; same pattern in avx2 and sse42 (read).** The index is relative to the string, not the document. Ran: `"abc\q"` at offset 12 gives index 3 on NEON, 17 on native.

**26. `ErrorType` is unequal to itself for some variants: `error.rs:107-151`.** Ran: `InputTooLarge`, `SimdUnsupported` and `Unexpected(None,None)` all compare `false` with themselves, so `Error` equality misbehaves too.

**27. Invalid literals are not classed as syntax errors: `error.rs:233-262`.** `ExpectedTrue`, `ExpectedFalse` and `ExpectedNull` are not in `is_syntax`. Ran: `nul` gives `is_syntax()=false`, `is_data()=true`.

**28. Equality depends on representation: `value/lazy.rs:43`, `value/tape/cmp.rs:11`.** Ran: a lazy tape value compared with the same value upgraded (no content change) is `false`. Tape comparison is key-order sensitive, while owned values compare equal.

**29. Map key gaps: `serde/de.rs:515-527`, `serde/value/owned/se.rs:378-544`, and the same in `serde/value/borrowed/se.rs`.**
- Ran: `HashMap<bool,_>` keys are rejected by `from_slice`, and `to_string` fails with `KeyMustBeAString` (serde_json accepts both directions).
- `to_owned_value` with a `u128` key fails even with `128bit`, while `to_string` succeeds.
- `HashMap<i128,_>` in `from_slice` fails without `128bit`, though `i128` values work.

**Findings: 29. Last `./clock` reading: 18 minutes used of 45.**
