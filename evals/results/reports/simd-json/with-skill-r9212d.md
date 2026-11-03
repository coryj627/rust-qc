Audit report: simd-json 0.16.0

Verdict: the stage 1 and stage 2 parser survived heavy fuzzing without a panic or an out-of-bounds read, but several number, string, serializer and soundness paths are wrong. The worst are silent wrong float values that depend on byte offset, a process abort from about 20 KB of nesting, safe-code reads past the tape, and invalid JSON written by the serializers.
Findings: 5 Blocker, 16 Major, 24 Minor. No text in the repository or its output addressed me.
I did not edit `crate/` by hand. Running its proptests appended to `crate/proptest-regressions/tests/serde.txt`. All probes ran from throwaway crates and patched copies beside it.

## Blocker

### Long floats decode to wrong values depending on their byte offset
Location: src/numberparse/correct.rs:168-172.
Problem: `start_digits` is set to the digit's byte value (`get!(buf, start) as usize`), not its index. Near offset 50-75 the corrected `digit_count` falls below 19, so the fast path uses the wrapped u64 significand. There is no error.
Input: `{"sensor":"a1","time":1700000000,"unit":"degreesN","lat":48.858369400000000000012}` gives lat = 0.011391092817107321. serde_json gives 48.8583694. The same number one byte earlier parses correctly.
Established: ran it through `to_owned_value` and `from_slice::<struct>`. With 20-29 digit floats at offsets 0..200, 3821 of 40000 were wrong.
Fix: `let start_digits = start;`.

### About 20 KB of nesting aborts the process
Location: src/value/owned.rs:307-330, src/value/borrowed.rs:394-420 and 456-482, src/value.rs:155-181, src/serde/de.rs:33-34, and write_json at src/value/owned/serialize.rs:87, src/value/borrowed/serialize.rs:87, src/value/tape/trait_impls.rs:737.
Problem: stage 2 has no depth limit and the DOM builders recurse, so hostile input kills the process with an uncatchable stack overflow.
Established: `[`x10000 + `]`x10000 aborts `to_owned_value` on a default spawned thread ("has overflowed its stack"), and 5000 is fine. The `{"a":` form behaves the same. On the 8 MB main thread, 100000 aborts `to_owned_value`, `to_borrowed_value` and `from_slice::<OwnedValue>`. `to_tape` handles 1000000 deep, but `encode()` on a 100000-deep tape aborts.
Fix: add a depth limit in stage 2 with a new error kind. serde_json's default is 128.

### Safe code can read past the end of the tape
Location: src/lib.rs:875-876 (`next_`), src/serde.rs:221-325 (`parse_*`), src/serde/de.rs:491, src/value/borrowed.rs:450, src/value/tape.rs:11 (public `Tape(pub Vec<Node>)`) and 41-53.
Problem: these use `get_kinda_unchecked`, which is `get_unchecked` in release builds, without a bounds check.
Established with three programs, each using only safe code:
- Calling `u8::deserialize(&mut de)` twice on a parsed `1`.
- `Tape::null().reset().deserialize::<u8>()`, which dies with "Trace/breakpoint trap" in release.
- `lazy::Value::from_tape(Tape(vec![Node::Array{len:3,count:3}]).as_value()).into_value()`, which printed `Array([Static(I64(0)), String(""), String("")])` from out-of-bounds memory in release.
All three are "Undefined Behavior" under `cargo miri run --release` and panic in debug.
Fix: use the checked `next()` in every `parse_*` and key path, and make the tape field private or validate tapes.

### x86 runtime fallback skips UTF-8 validation
Location: src/lib.rs:632, src/impls/native/mod.rs:10-30; the pre-validation exists only at src/lib.rs:672.
Problem: with the default `runtime-detection` feature on an x86 CPU without SSE4.2, the fallback calls `_find_structural_bits::<native>` directly. The native validator is a no-op whose comment says the input was pre-validated, but that happens only in the non-runtime path. Invalid bytes then reach `from_utf8_unchecked`, giving a `&str` holding invalid UTF-8 from a safe API.
Established: x86 cannot run here, so I read the code and simulated it. A copy forced to the native backend with the pre-validation removed returned Ok from `to_borrowed_value` on `["ab\xff\xfe"]`, with `&str` bytes [97,98,255,254]. CI's `-C target-feature` variants do not exercise this path when the host has AVX2.
Fix: call `core::str::from_utf8` in the runtime fallback, ideally inside the native path itself.

### NEON writes an over-aligned vector through a 4-aligned pointer
Location: src/impls/neon/stage1.rs:207-212 and 226.
Problem: `l % 4 == 0` tests the element index, not the address, so `std::ptr::write::<int32x4_t>` (align 16) targets a `Vec<u32>` that only guarantees align 4. This happens on any parse on aarch64.
Established: `cargo miri test --lib` stops here with Undefined Behavior, in `bad_dot`. Natively nothing visible happened: a global allocator returning blocks 4 bytes off a 16-byte boundary still parsed in debug and release, so this is UB by Miri, not an observed crash.
Fix: always use `write_unaligned` or `vst1q_s32`.

## Major

### Parsed strings are invalidated by the next string under Stacked Borrows
Location: src/stage2.rs:187-192.
Problem: `input.as_mut_ptr()` runs for every string and retags the whole `&mut [u8]`, invalidating every `&str` already on the tape.
Established: with the NEON write patched in a copy, Miri reports "trying to retag ... that tag does not exist in the borrow stack" in `test_deser_u16`. With `-Zmiri-tree-borrows`, 273 tests passed (prop and fuzz tests and `test_send_sync` skipped), and 1500 mutated documents in a release build were clean.
Fix: take the raw pointer once before the loop.

### Lone or mismatched surrogates become NUL or the wrong character
Location: src/stringparse.rs:61, 72, 77-80.
Problem: `Ok((0, src_offset))` is meant to signal an error, but code point 0 encodes to one byte, so the `o == 0` checks never fire. The low half is also not range-checked.
Established: `"\uD800"` gives `"\0"`, `"\uD800abc"` gives `"\0abc"`, and `"\uD800"` gives U+10400. All are Ok, on NEON and on a forced-native copy.
Fix: return an error for those cases and check `code_point_2 < 0xE000`.

### Huge exponents wrap and give plausible wrong values
Location: src/numberparse/correct.rs:186 (`exponent as i32`).
Established: `1e4294967296` gives 1.0, `1e4294967297` gives 10.0, `1e4294967295` gives 0.1, `1e-4294967296` gives 1.0, `1.5e4294967296` gives 1.5.
Fix: saturate or use `i32::try_from` and take the slow path.

### Trailing garbage after long numbers is accepted
Location: src/numberparse/correct.rs:174-177 and 190-191, with parse_large_integer at 239-244 and 281-302.
Problem: the slow float path and the large-integer path skip the structural check that shorter numbers get.
Established: `[1234567890123456789x]`, `12345678901234567890abc`, `[1.2345678901234567890123abc]` and `[1.2345678901234567890123.5]` all return Ok. `[15x]` errors. serde_json rejected the same fuzzed garbage.
Fix: check `is_structural_or_whitespace` on these paths too.

### NUL is treated as whitespace after scalars, hiding following bytes
Location: src/charutils.rs:4 and 15, src/numberparse.rs:33.
Problem: the tables allow byte 0 so the padding terminates numbers, but stage 1 does not treat NUL as whitespace, so nothing after it becomes structural.
Established: `1\0abc`, `true\0garbage`, `[1\0x]`, `[1\0,2]` and `{"a":null\0}` are all Ok. `"a"\0` gives TrailingData.
Fix: terminate on the real input length, or reject a NUL at an offset below `len`.

### Pretty serializer writes invalid JSON for tuple variants
Location: src/serde/se/pp.rs:236-243.
Problem: `end` writes `}` where it needs `]` then `}`, and an empty variant writes nothing.
Established: `to_string_pretty(&E::T(1,2))` gives `{\n  "T": [\n    1,\n    2\n  }`, and `E::T0()` gives `{\n  "T0": []`. serde_json rejects both. Later indentation drifts as well. Compact output is correct.
Fix: mirror the compact `end` and dedent twice.

### Empty unknown-length sequences and maps write unterminated JSON
Location: src/serde/se.rs:96-102, src/serde/se/pp.rs:134-141 and 521-527.
Problem: `end` closes only when `first` is false, but `serialize_seq(None)` has already written `[`.
Established: `collect_seq(vec.iter().filter(|_| false))` gives `[` in compact and pretty, and inside a struct `{"a":[,"b":2}`. `collect_map` with the same filter pretty-prints `{`.
Fix: write the closer when `len` was not `Some(0)`, tracked as compact SerializeMap's `wrote_closing` does.

### NaN and infinity serialize as huge numbers the crate then rejects
Location: src/serde/se.rs:637-642, src/serde/se/pp.rs:692-697, src/value/owned/serialize.rs:97 and 176, src/value/borrowed/serialize.rs:97 and 174, src/value/tape/trait_impls.rs:748 and 826.
Established: `to_string(&f64::NAN)` and `OwnedValue::from(f64::NAN).encode()` give `2.696539702293474e308`, infinity gives `1.797693134862316e308`. serde_json writes `null`, and this parser rejects numbers that large (`1.7976931348623159e308` gives InvalidNumber).
Fix: write `null` or return an error before calling `write_float`.

### Tape get_array and get_object drop the last node
Location: src/value/tape/trait_impls.rs:424-425, 453-454, 486-487, 518-519. Lazy values inherit this.
Problem: the slice uses `count`, but `as_array` and `as_object` use `count + 1`.
Established on `{"k":[1,2,3],"o":{"a":1,"b":2}}`: `get_array("k")` reports len 3, iterates [1,2], and `get(2)` is None. `get_object("o").get("b")` is None. For `{"k":[],"o":{}}`, `.len()` panics with "invalid tape array" / "invalid tape object". `get("k")` followed by `as_array()` works.
Fix: use `Node::count()`.

### Extra array elements are ignored and later data is misread
Location: src/serde/de.rs:233-244, 254-275, 322, 394-399.
Problem: nothing checks that the sequence was fully consumed.
Established:
- `(u8,u8)` and `[u8;2]` from `[1,2,3]` are Ok.
- `{"T":[1,2,3]}` gives `T(1,2)`.
- `Vec<(u8,)>` from `[[1,[2]],[9]]` gives `[(1,),(2,)]`, reading a nested value as the next element.
- `{"a":[1,2],"b":3}` into a one-tuple field gives a misleading ExpectedString.
serde_json rejects all of these.
Fix: skip the remainder or error when `len != 0` after `visit_seq`.

### Integer map keys fail when deserializing from a Value by value
Location: src/serde/value/owned/de.rs:189, src/serde/value/borrowed/de.rs:202.
Established: `from_owned_value::<HashMap<u8,u8>>(v)` and `from_borrowed_value` fail with `invalid type: string "1", expected u8`. `from_refowned_value`, `from_refborrowed_value` and `from_slice` work.
Fix: use MapKeyDeserializer, as ObjectRefAccess does.

### Duplicate keys: first wins in the DOM, last wins through serde, both are kept
Location: src/value/owned.rs:338-343, src/value.rs:186-193, src/value/borrowed.rs:488-494.
Established: `{"a":1,"a":2}` gives `get("a")` = 1, len 2, and `encode()` = `{"a":1,"a":2}`. With 40 keys plus repeats, `to_owned_value` keeps the first and `HashMap` via serde keeps the last. That is a parser differential on untrusted input. The README disclaims guarantees, but `insert` then updates only the first entry, and `{"a":1,"a":2}` != `{"a":2,"a":1}`.
Fix: make the checked insert the default.

### big-int-as-float still rejects part of the negative range
Location: src/numberparse/correct.rs:241.
Established with that feature: `-9223372036854775809` gives InvalidNumber while `-18446744073709551616` gives a float. The README promises this feature prevents such errors.
Fix: return `f64_from_parts_slow` under the feature.

### Some feature combinations do not build
Location: src/numberparse/approx.rs:339 and 2-3, src/numberparse/correct.rs:584.
Established:
- `--features 128bit,approx-number-parsing` fails: no `static_cast_i128` macro.
- `--no-default-features --features approx-number-parsing` fails: unresolved swar imports.
- `cargo test --features big-int-as-float` fails to compile its own test (E0277, Value vs StaticNode).
Fix: add the macro, gate the imports, and fix the test.

### Error positions are wrong or missing
Location: src/impls/neon/deser.rs:176, 179, 192, the same pattern in avx2:156-171, sse42:147-162, simd128:144-160, and src/lib.rs:834 for stage 1 errors.
Problem: SIMD string errors report the offset inside the string, where native reports `idx + 1 + src_i`. Stage 1 and serde type errors report 0. `Error::character` also casts one byte to `char`.
Established: `[1,2,"\x"]` gives index 0, `[1,2,3,4,5,6,"abc\uZZZZ"]` gives index 3, `[    "\u{1}"]` gives index 0, and a BOM reports `'ï'`. The field is documented as "Byte index it was encountered at".
Fix: add `idx + 1` and thread real offsets through.

### Errors do not let callers branch on the cause
Location: src/stage2.rs:404, 483, 531, 606; src/error.rs:96 and 221-223; src/lib.rs:29.
Established:
- `[] x`, `[1] 2` and `[]]` give `InternalError(TapeError)`, while `1 2` gives TrailingData.
- `[1,]`, `]` and `+1` are also "internal" errors.
- `InternalError` is public but unnameable (private module, not re-exported).
- Truncated `[1,` and `"abc` give Syntax with `is_eof()` false.
- `truex` has `is_data()` true.
- 12 ErrorType variants are never constructed, including UnterminatedString and ExpectedMapEnd.
Fix: return TrailingData, UnexpectedCharacter and Eof or UnterminatedString, and re-export or hide InternalError.

## Minor

### `-0` loses its sign
Location: src/numberparse/correct.rs:196. `-0` gives I64(0), while `-0.0` and serde_json give -0.0. Ran it. Fix: emit -0.0.

### Valid numbers with huge exponents are rejected
Location: src/numberparse/correct.rs:156-158. `0e99999999999` and `1e-99999999999` give InvalidNumber, while `1e-400` gives 0.0. Ran it. Fix: saturate the exponent.

### f32 widens and prints noise
Location: src/serde/se.rs:637-638, src/serde/se/pp.rs:692-693, src/value/owned/from.rs. `to_string(&0.1f32)` gives `0.10000000149011612`, serde_json gives `0.1`. Ran it. Fix: format with ryu's f32.

### Pretty output is misformatted though valid
Location: src/serde/se/pp.rs:556-560, 771-787, 711-720. Struct fields stay on one line (`{\n  "e": "U","x": 1\n}`), newtype variants are not indented, and bytes print `[\n  1,2\n]`. Ran it. Fix: add `new_line` and indent calls.

### Display prints Rust Debug for containers
Location: src/value/owned.rs:255-256, src/value/borrowed.rs:343-344, src/value/lazy.rs:134. `json!({"a":"x"})` displays `{"a": String("x")}`, and lazy prints raw tape nodes. Ran it. Fix: display `encode()`.

### ErrorType equality is not reflexive
Location: src/error.rs:107-151. `InputTooLarge == InputTooLarge` is false, and `SimdUnsupported` likewise (ran both). By reading, `Unexpected` has no arm either and any two `Io` compare equal. Fix: add the arms, or derive with Io handled separately.

### Error loses its cause and the enums cannot grow
Location: src/error.rs:265, 7; src/lib.rs:344. `source()` returned None on an Io error (ran it). ErrorType and Implementation are not `#[non_exhaustive]` (read). Fix: implement `source` and add the attribute.

### Empty tuple variants fail to round-trip through a Value
Location: src/serde/value/owned/de.rs:552-553 and 755-756, src/serde/value/borrowed/de.rs:565-566 and 778-779. `E::T0()` becomes `{"T0":[]}`, then fails with "invalid type: unit value". Ran it. Fix: `visit_seq` on the empty iterator.

### Equality depends on representation
Location: src/value/tape/cmp.rs:11, src/value/lazy.rs:43. Tape `{"a":1,"b":2}` != `{"b":2,"a":1}` while owned values compare equal. A tape lazy value != its upgraded form with the same content. Ran it. Fix: compare objects by key.

### Common map key types are unsupported
Location: src/serde/se.rs:298, src/serde/de.rs:515-527 and 575-578. `HashMap<bool,_>` fails both ways. `HashMap<i128,_>` and `HashMap<u128,_>` fail by default. serde_json handles them. Ran it. Fix: stringify and parse them. Float keys hit the same forwarding by reading; not run.

### f64 cannot read 128-bit integers
Location: src/serde.rs:331-338. With `128bit`, `from_slice::<f64>` on `18446744073709551616` gives ExpectedFloat. Ran it. Fix: add the I128 and U128 arms.

### fill_tape leaves a half-built tape on error
Location: src/lib.rs:144-147, src/stage2.rs:141 and 290. After `[1,` the tape holds `Array{len:0,count:0}, 1` and encodes as `[]`. Ran it. Fix: clear on error.

### A reset tape panics on ordinary calls
Location: src/value/tape/trait_impls.rs:123 and 817. `Tape::null().reset().as_value().value_type()` panics "invalid tape value", and `encode()` panics "invalid JSON". Ran it. Fix: return Null, or make the empty state unobservable.

### set_len over uninitialized bytes
Location: src/lib.rs:809-811. It breaks the `Vec::set_len` contract, behind `#[allow(clippy::uninit_vec)]`. Read; Miri did not flag it, because only written bytes are read. Fix: use a raw spare-capacity pointer.

### Zero-size allocation in a test
Location: src/lib.rs:1061, src/tests.rs:30. `AlignedBuf::with_capacity(0)` is UB under Miri ("creating allocation with size 0"). Only the test reaches it, since Buffers always adds padding. Fix: assert capacity is nonzero and change the test.

### approx-number-parsing is off well beyond "the edges"
Location: Cargo.toml feature comment, src/numberparse/approx.rs:81-213. 778926 of 2983248 random floats differ from std, including `-6.970748` and `6.04e-24`. Ran it. Fix: state the real accuracy in the docs.

### Big-endian targets look unsupported and nothing says so
Location: src/stage2.rs:18-23, 35-36, 62-63; src/numberparse.rs:118-122. Little-endian constants are compared against native reads, so `true` would be rejected. Read only. Fix: use `from_le_bytes`, or add a `compile_error!`.

### AVX2 path uses PCLMUL without detecting it
Location: src/impls/avx2/stage1.rs:61-71, src/lib.rs:380, 475, 624. A VM masking pclmulqdq but exposing avx2 would SIGILL. Read only. Fix: also detect pclmulqdq.

### Buffers::new panics on huge sizes
Location: src/lib.rs:113-115. `Buffers::new(usize::MAX - 20)` panics with add overflow in debug, with no `# Panics` section. Ran it. Fix: use checked or saturating add.

### Undocumented unsafe
Location: crate-wide. 312 unsafe blocks and 2 unsafe impls lack SAFETY comments; the worst are src/stage2.rs and src/lib.rs:1048-1049. Ran clippy. Fix: add justifications, starting with the paths above.

### Missing Debug
Location: 13 public types, including `Buffers` (src/lib.rs:90) and tape `Object` and `Iter`. Ran the lint. Fix: derive or implement Debug.

### Lint configuration
Location: src/lib.rs:1. 86 `#[allow]` lack a reason. `#![deny(warnings)]` breaks the build on this newer compiler (unused imports, and a new clippy lint). This is toolchain drift. `--all-features` cannot build on stable because of `hints`. Fix: use `#[expect(..., reason)]` and move the policy to CI.

### Glob re-exports and bare TODOs
Location: src/lib.rs:71, src/prelude.rs:2, src/serde/se.rs:3, src/serde.rs:12-13, src/value.rs:75; 14 TODO/FIXME without an owner, such as src/stage2.rs:13. Scan output, confirmed by reading. Fix: name the re-exports.

### Dead portable backend
Location: src/lib.rs:3 and 385-389, src/impls/portable/, Cargo.toml (`portable` is commented out). The cfgs can never be true, and `unexpected_cfgs` is allowed. Read. Fix: delete it or restore the feature.

## Verification

- `cargo fmt --all --check`: pass.
- `RUSTFLAGS="--cap-lints warn" cargo test --locked`: pass. Feature runs pass too, except big-int-as-float.
- `cargo clippy`, default features: fails only on drift (`deny(warnings)` plus a new lint).
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: pass.
- The skill's gate with `--all-features`: clippy, rustdoc and tests fail, from `hints` on stable and the missing macro above.
- `cargo +nightly-2026-10-01 miri test --lib`: fails at the NEON write. On a copy with the write patched, Stacked Borrows fails at the aliasing finding and Tree Borrows passes.
- Miri on x86_64: skipped, no target sysroot available. x86 findings are from reading, plus the native simulation.
- Differential against serde_json with bounds checks on: 245411 exhaustive short inputs and 1000000 mutations gave no panics. The only divergences outside the reported classes were `-0`; inputs with NUL, `\u` or long digit runs were excluded from the comparison.
- The same harness on a forced-native copy: 400000 mutations, no panics and no divergence outside those classes.
- Floats at offset 0 against `str::parse`: 0 wrong of 2983248, default number parser.

Number of findings: 45. Last `./clock` reading: 23 minutes used of 45.
