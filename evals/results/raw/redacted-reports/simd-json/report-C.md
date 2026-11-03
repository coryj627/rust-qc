# Audit report: simd-json 0.16.0 (`crate/src/`)

**How it was tested**
- All code ran through `./run` on aarch64, which uses the NEON code path.
- `crate/` was not modified. Probes lived in `tmp/`.
- Two patched copies were used:
  - `tmp/cm` removes `#![deny(warnings)]` for Miri, and later also forces unaligned writes to get past finding 14.
  - `tmp/cn` routes aarch64 through the `native` implementation.
- The existing test suite passes with default features (313 lib tests, 126 integration tests).
- Differential fuzz:
  - 400k numbers at low input offsets against `str::parse::<f64>` found no float mismatch.
  - 300k mutated documents against serde_json, on both NEON and the patched native path, found only the differences reported below, plus intended ones such as big-integer rejection.
- x86 and wasm code could not be run. Findings based only on reading are marked.
- No text in the repository addressed me as instructions.

## High: wrong data, memory safety, crashes

**1. `numberparse/correct.rs:168-172`: long floats silently get a wrong value, depending on their position in the document.**
- `start_digits` is set to the byte value at `start`, not the index. The later subtraction therefore gives a meaningless `digit_count`.
- If the number starts about 20 bytes past the digit's ASCII code, the slow path is skipped and the wrapped `u64` significand is used.
- Ran: `{"id":12345,"name":"sensor-abcdefghijklmnopqrstuvw","reading":3.14159265358979323846264338}` gives `"reading":1.6610052918127953e-7`. serde_json gives `3.141592653589793`.
- `1.2345678901234567890123` parses correctly at offsets 0-50 and 80+, and gives `0.00048...` at offsets 55-70.

**2. `stringparse.rs:58-62, 71-73`: a lone high surrogate decodes to U+0000 instead of failing.**
- `Ok((0, offset))` is meant as "invalid", but `codepoint_to_utf8(0)` writes a NUL and returns 1, so the `o == 0` check never fires. The native path also writes the NUL.
- Ran: `"\uD83D"` gives `"\0"`, `"\uD83Dabc"` gives `"\0abc"`, and `{"\uD800":1}` gives the key `"\0"`. serde_json rejects all of them.
- Callers receive injected NUL bytes from invalid input.

**3. `stringparse.rs:74-80`: the low surrogate has no upper bound.**
- Only `checked_sub(0xdc00)` is applied, so anything at or above U+DC00 is accepted.
- Ran: `"\uD800"` gives `U+10400`, and `"\uD800￿"` gives `U+123FF`. serde_json rejects both.

**4. `numberparse/correct.rs:186` (with the limit at `:156`): the exponent is truncated by `exponent as i32`.**
- The loop allows exponents above 2^32.
- Ran: `1e4294967296` gives `1.0`, `1e4294967297` gives `10.0`, `1.5e4294967300` gives `15000.0`, and `1e-4294967296` gives `1.0`.
- serde_json reports out of range for the positive cases and returns `0.0` for the negative one.

**5. `serde.rs:221-332`, `serde/de.rs:491`, `lib.rs:875`, `value/borrowed.rs:456-491`: undefined behaviour is reachable from safe code through `next_()`.**
- `parse_u8` through `parse_double` call the unchecked `next_()` with no bounds check.
- These safe calls all reach an out-of-bounds tape read:
  - `u8::deserialize(&mut d)` called twice on a one-value tape.
  - `Tape(vec![]).deserialize::<u8>()` (the tape field is public).
  - A `Visitor` calling `next_value` past the end of the map.
  - `lazy::Value::from_tape(malformed).into_value()`.
- Miri `--release` on `tmp/cm` reports "`assume` called with `false`" at `safer_unchecked.rs:25` for all four. A debug build panics on the double-deserialize case.

**6. `lib.rs:613-635` with `impls/native/mod.rs:14-30`: invalid UTF-8 is accepted on the runtime fallback (x86 without SSE4.2).**
- The native validator always returns Ok. It relies on the `from_utf8` pre-check at `lib.rs:672`, but that check exists only in the non-runtime variant of `find_structural_bits`.
- With default features on such a CPU (for example QEMU's `qemu64`), `from_utf8_unchecked` builds an invalid `&str`.
- Not run on x86. Simulated with the native path in `tmp/cn`: `"a\xffb"` was accepted and returned bytes `[97,255,98]`.

**7. `value/owned.rs:307-349`, `value/borrowed.rs:394-431, 456-493`, `value.rs:155-200`, `serde/de.rs:33-34`, tape `write_json`: no depth limit, so deep nesting aborts the process.**
- Ran in release: 100k levels of `[`/`]` (a 200 KB document) aborts with "has overflowed its stack" (exit 134). This happens in `to_owned_value`, `to_borrowed_value`, `from_slice::<OwnedValue>`, `from_slice::<IgnoredAny>` and tape `encode`. 1,000 levels works.
- An attacker can crash the process with a small input.

**8. `value/owned.rs:340`, `value/borrowed.rs:429, 491`, `value.rs:191`: `unsafe insert_nocheck` is called on untrusted keys.**
- Its uniqueness precondition is not checked by these safe functions, and `Cargo.toml` itself calls duplicate keys "undefined behavior".
- Ran on `{"a":1,"a":2}`: `len()` is 2, `get` returns the first value, `encode` emits both, and `remove` leaves the second one reachable.
- `from_slice::<OwnedValue>` uses `insert` (last value wins), so it disagrees with `to_owned_value` on the same input.
- Miri under Tree Borrows passed 100 duplicate keys, so I saw no memory fault, only wrong behaviour and a broken contract.

**9. `value/tape/trait_impls.rs:424, 453, 486, 518`: tape `get_array`, `get_object`, `try_get_array` and `try_get_object` return a slice one node short.**
- They use `array_count()`/`object_count()`, which exclude the header. `as_array` correctly adds 1. Lazy values inherit the bug.
- Ran:
  - On `{"a":[1,2,3]}`: `len()` is 3, `iter` yields 2 items, and `get(2)` is `None`.
  - On `{"o":{"x":1,"y":2}}`: keys are `["x"]`, and `y` is `None`.
  - On `{"n":[[1,2]]}`: iterating panics with `mid > len` at `array.rs:14`.
  - On a nested object: panic at `object.rs:91`.

**10. `serde/se/pp.rs:236-243`: `to_string_pretty` produces invalid JSON for tuple variants.**
- The non-empty arm writes `}` with no `]` and dedents once.
- The empty arm writes nothing.
- Ran:
  - `E::Tv(1,2)` gives `{ "Tv": [ 1, 2 }`, and re-parsing fails.
  - `E::Empty()` gives `{ "Empty": []`.
  - Indentation of later siblings is also wrong.

**11. `serde/se.rs:642` (`:638`), `serde/se/pp.rs:693, 697`, `value/owned/serialize.rs:97, 176`, `value/borrowed/serialize.rs:97, 174`, `value/tape/trait_impls.rs:748, 826`: non-finite floats serialize as bogus numbers.**
- These call `write_float`, which uses ryu `format_finite`, without a finiteness check.
- Ran: NaN gives `2.696539702293474e308`, Inf gives `1.797693134862316e308`, and -Inf gives `-1.797693134862316e308`. `Value::encode` behaves the same way.
- serde_json writes `null`. These outputs are not the original value, and they are above f64 max, so they overflow when read back.

**12. `serde/se.rs:96-102, 128-134, 160-166`, `serde/se/pp.rs:134-141, 521-528`: empty unknown-length containers produce truncated JSON.**
- `end()` skips the closer when `first` is true, assuming `[]` or `{}` was already written.
- Ran:
  - `collect_seq` over an empty filtered iterator gives `[`, in both compact and pretty.
  - Pretty `collect_map` over the same gives `{`.
  - A struct with an empty `#[serde(flatten)]` map gives `{` in pretty.
- serde_json gives `[]` and `{}`. Compact maps are correct because they track `wrote_closing`.

## Medium

**13. `numberparse/correct.rs:174-177, 191, 208-245, 250-303`: the slow number paths have no trailing-byte check.**
- Ran: `1.00000000000000000000000x` gives `1.0`, `[1.00000000000000000000000x]` is accepted, `12345678901234567890x` is accepted as U64, and `[1234567890123456789x]` is accepted. serde_json rejects all of them.

**14. `impls/neon/stage1.rs:207-212, 226`: misaligned `ptr::write` on every parse on aarch64.**
- `l % 4 == 0` tests the length, not the address. The code then uses an aligned write of `int32x4_t` into a `Vec<u32>` that only guarantees alignment 4.
- Miri on any `to_owned_value` reports "alignment 4, but alignment 16 is required".
- Common allocators hide this in practice. It is still UB on every call.

**15. `numberparse.rs:76-109`, `impls/avx2/stage1.rs:61-71`, `lib.rs:380, 475, 624`: x86 instructions are used without feature gating (read only, not run).**
- `parse_eight_digits_unrolled` uses SSSE3 and SSE4.1 intrinsics with no `target_feature` and no detection, even when runtime detection fell back to native. A float with 8 or more fractional digits can hit an illegal instruction on an old CPU.
- AVX2 `compute_quote_mask` calls `_mm_clmulepi64_si128`, but only AVX2 is detected or enabled.

**16. `charutils.rs:4, 15`, `numberparse.rs:32`, `stage2.rs:24, 53, 73`: NUL counts as a valid token terminator, so anything after it is ignored.**
- Ran: `123\0abc` gives `123`, `true\0abc` gives `true`, `[1\0,2]` gives `[1,2]`, and `84.544\04` gives `84.544`.
- Stage 1 never marks the text after NUL as structural.
- A unit test at `correct.rs:482` already carries a todo about this.

**17. `serde/de.rs:254-262, 398, 433-438`: fixed lengths are not enforced, which can misalign the tape.**
- Ran:
  - `(u8,u8)` from `[1,2,3]` gives `(1,2)`.
  - `{"D":[1,2,3]}` gives `D(1,2)`.
  - `((u8,),u8)` from `[[1,2],3]` gives `((1,),2)`, which takes a value from the wrong array.
  - `{"a":[1,2,7],"b":9}` into `struct W { a: T2, b: u8 }` fails with a misleading `ExpectedString`.
- serde_json reports trailing characters. Deserializing from an owned value also ignores the extra element.

**18. `serde/value/owned/de.rs:189`, `serde/value/borrowed/de.rs:202`: integer map keys fail when deserializing from an owned value.**
- Keys are passed as `Value::String(k)` instead of through `MapKeyDeserializer`.
- Ran: `HashMap<u8,u8>` from `{"1":2}` errors with "invalid type: string" from `OwnedValue` and `BorrowedValue` by value, but succeeds from `&OwnedValue`, `&BorrowedValue` and `serde_json::Value`.

**19. `stage2.rs:188`: Stacked Borrows violation for documents with two or more strings.**
- Each `input.as_mut_ptr()` call reborrows `&mut` and invalidates earlier returned `&str`s.
- Miri (default mode): "tag does not exist in the borrow stack". The `stage2.rs:188` retag is the invalidating one, and it fires at `lib.rs:875`.
- The same test passes under `-Zmiri-tree-borrows`, so this is a violation of an experimental aliasing model.

**20. `numberparse/approx.rs:200, 211, 533-535` (feature `approx-number-parsing`): infinity is accepted.**
- No finiteness check.
- Ran: `2e308` and `1.7976931348623159e308` give `F64(inf)`. The default parser errors on both.
- Related behaviour under the same feature:
  - `0e0` gives `U64(0)`, not a float.
  - `1e-324` and `4.9e-324` are rejected with `InvalidExponent`.

**21. `serde.rs:331-338`, `value/tape/trait_impls.rs:50-82`, `serde/value/*/se.rs` key serializers (feature `128bit`): 128-bit support is incomplete.**
- Ran:
  - `f64` from a 128-bit integer gives `ExpectedFloat`.
  - The tape value for `1267650600228229401496703205376` reports `value_type=U128`, but `as_u128`, `as_i128` and `cast_f64` are all `None`, while the owned value returns `Some`.
  - An `i128` or `u128` map key through `to_owned_value` errors with "not supported", while `to_string` works.

## Low

**22. `numberparse/approx.rs:7, 339`: the build fails with `approx-number-parsing` and `128bit` together.**
- `cargo check` reports "cannot find macro `static_cast_i128`".

**23. `impls/neon/deser.rs:176, 179, 192` (also `avx2:156-171`, `sse42:147-162`, `simd128:144-160`, `portable:134-149`): string error offsets are wrong.**
- They report `src_i`, which is relative to the string, and the escape case also omits `bs_dist`. Native uses `idx + 1 + src_i`.
- Ran on NEON: a bad escape at byte 16 reports index 3, while the native path reports 17.

**24. `numberparse/correct.rs:155-160`: valid large exponents are rejected.**
- Ran: `0e99999999999999` gives `InvalidNumber`. serde_json gives `0.0`.

**25. `serde/se/pp.rs:557, 607`: pretty struct fields after the first have no newline.**
- Ran: `{\n  "a": 1,"b": 2\n}`. The JSON is valid but the format is wrong. Struct variants have the same problem.

**26. `error.rs:107-152`: `ErrorType` equality is not reflexive for some variants.**
- Ran: `InputTooLarge == InputTooLarge`, `SimdUnsupported == SimdUnsupported` and `Unexpected(None,None) == Unexpected(None,None)` are all false.

**27. `error.rs:221-223, 234-262`, `lib.rs:172`: errors are misclassified.**
- Ran:
  - `[tru]` gives `ExpectedTrue` with `is_syntax()` false and `is_data()` true.
  - Truncated `[1,` and `"abc` give `Syntax` with `is_eof()` false.

**28. `serde/se.rs:638`, `serde/se/pp.rs:693`: `f32` is widened before printing.**
- Ran: `0.1f32` gives `0.10000000149011612`. serde_json gives `0.1`. It round-trips, but it is noisy.

**29. `value/lazy.rs:43`, `value/tape/cmp.rs:10`: equality is inconsistent between representations.**
- Ran: a lazy tape value does not equal the same value after upgrading to borrowed. Tape `==` is sensitive to key order, while owned `==` is not.

**30. `value/owned.rs:250-258` (also `value/borrowed.rs`): `Display` prints Rust Debug for containers.**
- Ran on the owned value: `{"a": Array([Static(U64(1)), String("x")]), "b": Static(Null)}`.
- The borrowed `Display` was only read.

**31. `lib.rs:791, 810`: `string_buffer.set_len` on uninitialized bytes (read only).**
- The buffer is then passed as `&mut [u8]`, which breaks the `Vec::set_len` contract. Miri did not flag it in my runs.

**32. `numberparse/correct.rs:582-589`: unit tests do not compile with `big-int-as-float`.**
- Ran: `cargo test --lib --features big-int-as-float` fails with E0277.

**33. `lib.rs:1, 47`, `stage2.rs:3`, `numberparse/correct.rs:12`: `#![deny(warnings)]` with unused imports breaks plain builds on this compiler.**

**Number of findings: 33. Last `./clock` reading: 17 minutes used of 45.**
