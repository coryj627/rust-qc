# Audit report: simd-json 0.16.0

## Verdict

The parser's core stage 1 and stage 2 logic held up: millions of differential inputs against serde_json and std float parsing produced no panics. The defects are around that core. They include silent wrong floats, a stack-overflow abort from small inputs, undefined behavior reachable from safe code, invalid JSON from the serializers, and tape accessors that truncate data.

Findings: 5 Blocker, 22 Major, 18 Minor, 2 Note.

I did not change `crate/`. Probes ran from throwaway crates in `tmp/`. Three Miri runs and the native-fallback fuzz used patched copies, marked below. x86 and wasm code cannot run on this aarch64 container, so I only read it. No repository text or program output addressed me.

## Blocker

### Stack overflow aborts the process on small nested input
Location: src/value/owned.rs:307-349, src/value/borrowed.rs:394-438, src/value.rs:155-200, plus serde `deserialize_any` (src/serde/de.rs:33-34) and tape encode (src/value/tape/trait_impls.rs:816).
Problem: there is no depth limit and every DOM builder recurses. `[` x N + `]` x N through `to_owned_value` killed the process with SIGABRT.
- Release build, 8 MB main thread: N=20,000 passed, N=50,000 aborted (100 KB of input).
- Release build, 2 MB thread: N=5,000 passed, N=10,000 aborted (20 KB of input).
- Also aborted at N=1,000,000 in a dev build: `to_borrowed_value`, `from_slice::<OwnedValue>`, and nested objects at N=300,000.
- In a release build `to_tape` succeeded at N=1,000,000 and then `encode()` aborted.
- serde_json returns a recursion-limit error on the same input.

Established by running it. A caller cannot catch this.
Fix: add a depth limit returning an error, checked in stage 2 where the depth is already tracked, or build the DOM iteratively.

### Long floats silently parse to a wrong value, depending on byte offset
Location: src/numberparse/correct.rs:168-172.
Problem: `let start_digits = get!(buf, start) as usize` stores the byte value at `start`, not the index. `digit_count` is then corrupted. When the number starts in a 19-byte window near `digit value + digit_count`, a number with 20 or more significant digits takes the fast path with a wrapped `num`.
Results from running it:
- `[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,1.1,1.2,1.3,123456789012345678901.5]` gives 1.7082781258626383e18. serde_json gives 1.2345678901234567e20.
- `1.2345678901234567890123` after 55 to 70 spaces gives 0.0004807115922877859. At other offsets it is correct.

In the differential fuzz the shortest retained sample of the float-difference category, on three separate runs, showed this signature. The run with 1.5M generated documents, a quarter of them padded to offsets 80-159, recorded 16,351 differences from std. I did not check every one against the window.
Fix: `let start_digits = start;`.

### Safe code can read past the end of the tape
Location: src/lib.rs:875-879 (`next_`), src/serde.rs:220-339 and src/serde/de.rs:491 (callers), src/value/tape.rs:11 and 41-53, src/value/borrowed.rs:449-463, src/value/lazy.rs:64 and 112.
Problem: these safe paths reach an unchecked `get_unchecked`:
- `u64::deserialize(&mut d)` twice on `1`.
- `Tape::null().reset().deserialize::<u64>()`.
- `lazy::Value::from_tape(Tape(vec![Node::Array{len:1,count:0}]).as_value()).into_value()`. `Tape`'s field and `Node` are public.
- An empty tape left by a failed `fill_tape`.

Debug builds panic with index out of bounds. `cargo miri run --release` reports `Undefined Behavior: assume called with false` in `Deserializer::next_` and `BorrowSliceDeserializer::next_`. Miri ran the first three cases (the reset tape, the double deserialize and the hand-built tape); the fill_tape case was only run in a debug build, where it panicked.
Fix: use the checked `next()` in all serde scalar paths and in `BorrowSliceDeserializer`. Make `Tape`'s field private or validate tapes.

### Misaligned SIMD write on every aarch64 parse
Location: src/impls/neon/stage1.rs:207-212 and 226.
Problem: `flatten_bits` picks `ptr::write` when `l % 4 == 0`, but the target type is `int32x4_t` (16-byte alignment) inside a `Vec<u32>`. Index alignment says nothing about address alignment. Miri stops on the first parse: `accessing memory based on pointer with alignment 8, but alignment 16 is required`. Ordinary hardware tolerates the write, but it is undefined behavior from any safe parse call. The other SIMD backends use unaligned stores (read only).
Fix: always use `write_unaligned`.

### Tape `get_array` and `get_object` drop the last node and can panic
Location: src/value/tape/trait_impls.rs:424-425, 453-454, 486-487, 518-519. Lazy values inherit this through src/value/lazy/trait_impls.rs:529-550.
Problem: the slice uses `idx..idx + count` and leaves out the `+ 1` that `as_array` applies. Run on `{"a":[1,2,3],...}`:
- `get_array("a")` reports len 3 but iterates only 1, 2, and `get(2)` is None.
- `get_object("b")` loses key `y`.
- `{"c":[1,[2,3]]}` panics with `mid > len` when iterated.
- `{"a":[]}` panics with `invalid tape array` on `.len()`.
- Lazy `get_array` truncates the same way, and lazy `get_object(...).get("y")` panics.

Fix: slice `idx..idx + count + 1`, or reuse `Node::count()`.

## Major

### Huge exponents wrap to small ones
Location: src/numberparse/correct.rs:186.
Problem: `exponent as i32` truncates. Run results: `1e4294967296` gives 1.0, `1e4294967297` gives 10.0, `1e-4294967297` gives 0.1, `1.5e4294967298` gives 150.0. The first, second and fourth should be rejected as out of range, the same as `1e400`, and the third should be 0.0.
Fix: clamp to i32 range before the cast.

### Trailing garbage accepted after long numbers
Location: src/numberparse/correct.rs:173-178 and 190-191.
Problem: the large-integer and slow-float paths return before the trailing-character check. Run results: `1234567890123456789abc`, `[1234567890123456789abc]`, `1.2345678901234567890123abc` and `1234567890123456789012.5xyz` all parse successfully. Short numbers with the same suffix are rejected.
Fix: check the terminator before branching.

### Invalid surrogate escapes decode to wrong text
Location: src/stringparse.rs:58-62, 71-73, 77-80.
Problem: `Ok((0, offset))` is meant to signal failure, but code point 0 encodes as one NUL byte and `o == 0` never fires. The low surrogate range is also not checked. Run results, all rejected by serde_json:
- `"\ud800"` gives `"\0"`.
- `"\ud83dxy"` gives `"\0xy"`.
- `"\ud800"` gives U+10400.
- `"\ud800￿"` gives U+123FF.

Fix: return `Err` for a missing or out-of-range low surrogate.

### Pretty printer emits invalid JSON for tuple variants
Location: src/serde/se/pp.rs:236-243.
Problem: `end` writes `}` without `]`, and leaks one indent level. `to_string_pretty(&V::T(1,2))` gives `{ "T": [ 1, 2 }`, and reparsing returns `ExpectedArrayContent`. A zero-length tuple variant gives `{ "T0": []` with no closing brace. Established by running it and comparing with serde_json.
Fix: write `]`, dedent, then write `}`, as the compact serializer does.

### Empty sequences of unknown length produce unterminated JSON
Location: src/serde/se.rs:96-102 and 731-741; src/serde/se/pp.rs:134-141 and 521-528.
Problem: `serializer.collect_seq(iter.filter(..))` with no items gives `[` in both compact and pretty output. Pretty `collect_map` with no items gives `{`. serde_json gives `[]` and `{}`. Established by running it. `end` skips the closer when `first` is true even though the opener was already written.
Fix: track whether the closer was written, as the compact map already does.

### NaN and infinity serialize as wrong finite numbers
Location: src/serde/se.rs:637-643, src/serde/se/pp.rs:693-697, src/value/owned/serialize.rs:97 and 176, src/value/borrowed/serialize.rs:97 and 174, src/value/tape/trait_impls.rs:748 and 826.
Problem: `write_float` uses `format_finite` on non-finite values. `to_string(&vec![NAN, INF, -INF])` gives `[2.696539702293474e308,1.797693134862316e308,-1.797693134862316e308]`, where serde_json gives nulls. The NaN text cannot be reparsed by simd-json itself (`InvalidNumber`). Established by running it.
Fix: write `null` or return an error for non-finite values.

### Deserializer ignores extra elements and misaligns on them
Location: src/serde/de.rs:238-240, 254-262, 322, 425-439; src/serde.rs:56-62 (no end-of-tape check).
Problem: a visitor that stops early leaves the tape cursor inside the array, and the outer visitor keeps reading from there. Run results:
- `[1,2]` as `(u8,)` gives `Ok((1,))`.
- `[1,2,3]` as struct `S{a,b}` gives `Ok`.
- `[[1,[7]],[3,4]]` as `Vec<(u8,)>` gives `[(1,),(7,)]`, data from the wrong position.

serde_json rejects trailing elements in such cases. I ran serde_json only on the surrogate and pretty cases, so that comparison is from its documented `end_seq` behavior.
Fix: skip remaining nodes using `count`, or error when the sequence is not exhausted.

### Duplicate keys break an `unsafe` contract and give inconsistent results
Location: src/value/owned.rs:338-341, src/value/borrowed.rs:427-430, src/value.rs:189-192.
Problem: untrusted input reaches `unsafe { insert_nocheck }`, whose safety contract requires unique keys. The README calls the result undefined behavior, yet the entry points are safe. Run on `{"a":1,"a":2}`: `v["a"]` is 1, `len()` is 2, and `to_string` emits both keys. The same document through `from_slice::<HashMap>` keeps the last value, and a derived struct errors. A first-wins DOM against a last-wins serde path invites parser-differential bugs.
Fix: make `insert` the default, or check for duplicates and return an error.

### NUL accepted after scalars
Location: src/charutils.rs:4 and 15 (index 0), src/numberparse.rs:33.
Problem: NUL is treated as a valid terminator. `123\0`, `[123\0]`, `true\0`, `[null\0,1]` and `1.5\0` all parse, and serde_json rejects them. The fuzz found only this category of false acceptance, among the inputs it generated. The test at src/numberparse/correct.rs:480-485 asserts this wrong behavior, with a comment saying it should fail.
Fix: treat NUL as invalid and compare the position against the real input length.

### Error index is wrong for errors inside strings
Location: src/impls/neon/deser.rs:176, 179, 192. Same pattern in avx2, sse42, simd128 and portable. Native is correct at src/impls/native/deser.rs:45.
Problem: `index()` is documented as a byte index, but these report `src_i`, relative to the string. `["aaa...48 chars","\q"]` reports index 0 instead of about 53. Established by running it on NEON and reading the others.
Fix: report `idx + src_i`.

### `from_owned_value` and `from_borrowed_value` reject integer map keys
Location: src/serde/value/owned/de.rs:189, src/serde/value/borrowed/de.rs:202.
Problem: keys are passed as `Value::String`. A `HashMap<u32,u32>` round trip fails with `invalid type: string "1", expected u32`. The `from_ref*` variants succeed. Established by running it.
Fix: use `MapKeyDeserializer` in both.

### `BorrowedValue` cannot deserialize non-borrowable keys
Location: src/serde/value/borrowed/de.rs:483.
Problem: `next_key::<&str>()` fails on any key the source cannot lend. `BorrowedValue::deserialize` from serde_json on `{"a\n":1}` gives `expected a borrowed string`. `OwnedValue` works on the same input. Established by running it.
Fix: use `Cow<str>` or a key visitor that accepts both.

### Parsed strings are invalidated under Stacked Borrows
Location: src/stage2.rs:188.
Problem: `input.as_mut_ptr()` is reborrowed per string, which invalidates earlier `&str` values on the tape. Miri on a copy with the NEON write patched reports a retag error at `Node::as_str`. Tree Borrows passes on the same program, and the crate's lib tests also pass under Tree Borrows on that copy, with proptest and `test::parsing` skipped. This is a real violation of one aliasing model, not an observed miscompile.
Fix: take the raw pointer once before the loop.

### x86 code runs instructions runtime detection does not check
Location: src/numberparse.rs:85-109, src/impls/avx2/stage1.rs:65, src/lib.rs:380-383, 475-477, 624-627.
Problem: the default `swar-number-parsing` path calls SSSE3 and SSE4.1 intrinsics with no target feature and no detection, even in the fallback the README promises. The AVX2 path uses PCLMULQDQ while checking only `avx2`. On a CPU or VM without those features the process can hit an illegal instruction. Read only, because this cannot run on aarch64.
Fix: use the SWAR routine outside detected paths and add `pclmulqdq` to the detection.

### `is_eof` and `is_syntax` misclassify common errors
Location: src/error.rs:221-263, src/lib.rs:1011-1013, src/stage2.rs:172.
Problem: run results:
- `[1`, `{"a":1` and `"abc` give `Syntax`, with `is_eof() == false`. Only empty input counts as EOF.
- `tru`, `[truX]` and `nul` give `is_syntax() == false` and `is_data() == true`.

Fix: use `Eof` for truncation, and classify malformed literals from stage 2 as syntax errors.

### Ordinary malformed JSON is reported as an internal error
Location: src/stage2.rs:404, 483, 531, 606; src/error.rs:96; src/lib.rs:69.
Problem: `[1,]`, `x`, `[1]x`, `{}x` and `{"a":}` return `InternalError(TapeError)`, while `1 x` style scalars use `TrailingData`. `InternalError` is not re-exported, so callers cannot name it. The trailing-data and bad-value cases were run; `1 x` was read from the code path.
Fix: use specific error kinds and re-export the type or remove it.

### `ErrorType` equality is not reflexive
Location: src/error.rs:107-152.
Problem: `InputTooLarge`, `SimdUnsupported` and `Unexpected` are missing from the match. `ErrorType::InputTooLarge == ErrorType::InputTooLarge` printed false for all three. A caller comparing errors never matches.
Fix: add the arms, or derive equality with Io handled separately.

### `128bit` plus `approx-number-parsing` does not compile
Location: src/numberparse/approx.rs:7 and 339.
Problem: `cargo check --features 128bit,approx-number-parsing` fails with `cannot find macro static_cast_i128`. This also breaks `--all-features`.
Fix: import the macro.

### Tape values lose 128-bit numbers
Location: src/value/tape/trait_impls.rs:48-81.
Problem: with `128bit` on, a U128 tape value gives `as_u128()` None, `as_i128()` None, `cast_f64()` None and `is_u128()` false, while `value_type()` is U128. `OwnedValue` returns the number. A negative I128 tape value gives None too. Established by running it.
Fix: override these methods on the tape value.

### `big-int-as-float` still rejects some integers
Location: src/numberparse/correct.rs:241.
Problem: `-9223372036854775809` returns `InvalidNumber`, while `-18446744073709551616` and `18446744073709551616` become floats. Established by running it.
Fix: fall back to the slow float path on that arm under the feature.

### Test suite does not compile with `big-int-as-float`
Location: src/numberparse/correct.rs:584-587.
Problem: `cargo test --features big-int-as-float` fails with `can't compare owned::Value with StaticNode`. The feature's own test cannot run.
Fix: compare against `Static(StaticNode::from(..))`.

### `approx-number-parsing` rejects valid floats
Location: src/numberparse/approx.rs:398-409.
Problem: the integer-part overflow check runs before the parser knows the number is a float. `18446744073709551616.5` and `18446744073709551617e2` return `Overflow`, while `12345678901234567890123.5` succeeds. Established by running it.
Fix: on overflow, restart with `parse_float` if a fraction or exponent follows.

## Minor

### Long exponents rejected where the value is representable
src/numberparse/correct.rs:155-158. `0e99999999999` and `1e-99999999999` return `InvalidNumber`, where 0.0 is correct. Run. Fix: saturate the exponent instead of erroring.

### Tape and lazy equality disagree with the DOM
src/value/tape/cmp.rs:10-12, src/value/lazy.rs:43. Reordered keys compare unequal on tapes and equal on `OwnedValue`. A lazy tape and an upgraded lazy value with the same content compare false. Run. Fix: compare structurally.

### `Display` of values prints Debug output
src/value/owned.rs:255-256, src/value/borrowed.rs:343-344, src/value/lazy.rs:134. Output looks like `{"a": Array([Static(U64(1)), ...` and strings are unquoted. Run on owned; borrowed and lazy read only. Fix: print JSON.

### Zero-field tuple variant fails through values
src/serde/value/owned/de.rs:552-553 and 756, src/serde/value/borrowed/de.rs:566 and 779. `E::T0()` fails with `invalid type: unit value` while `from_slice` works. Run. Fix: call `visit_seq` for empty arrays.

### 128-bit handling is inconsistent
src/serde/de.rs:524-527, src/serde/value/owned/se.rs:372-544, src/serde.rs:320-339.
- Without the feature, `to_string` accepts u128 map keys and `from_slice` rejects them.
- `18446744073709551615` as `i128` returns `ExpectedSigned`.
- With the feature, `to_owned_value` rejects u128 keys, and `f64` rejects U128 values.

Run. Fix: support these on both sides.

### f32 is widened before printing
src/serde/se.rs:637-639, src/serde/se/pp.rs:693. `0.1f32` prints as `0.10000000149011612`, where serde_json prints `0.1`. Run. Fix: format as f32.

### Pretty printer omits newlines between struct fields
src/serde/se/pp.rs:557 and 607. Output is `{\n  "a": 1,"b": 2\n}`. The JSON is valid but the formatting is wrong. Run for structs, read for struct variants. Fix: call `new_line()` after the comma.

### Errors lack positions and causes
src/lib.rs:762 and 834, src/error.rs:265. UTF-8, control-character, unterminated-string and all serde shape errors report index 0. `source()` returns None for I/O errors. Run. Fix: carry positions and implement `source`.

### Failed `fill_tape` leaves a misleading tape
src/lib.rs:144-147, src/value/tape/trait_impls.rs:739 and 817. `[1,2,[3` returns Err but leaves a partial tape that encodes as `[]`. An empty tape panics in `encode`. Run. Fix: clear on error and document it.

### Zero-size allocation in `AlignedBuf`
src/lib.rs:1060-1063, src/tests.rs:30. `with_capacity(0)` is undefined behavior and stops the crate's own tests under Miri. Production callers always pass at least 64 (read). Fix: guard against zero.

### Error path indexes the wrong slice
src/numberparse/correct.rs:375 and 395. `get!(slice, offset)` uses an absolute offset on a subslice. I could not reach it, so this is read only. Fix: use index 0.

### `approx-number-parsing` returns integers for `1e0`
src/numberparse/approx.rs:516. `1e0` gives U64(1) and `0e0` gives U64(0). Run. Fix: track "has exponent" separately.

### Build fails on current compilers from `deny(warnings)`
src/lib.rs:1. This is toolchain drift: new unused-import warnings at src/numberparse/correct.rs:12, src/lib.rs:47 and src/stage2.rs:3. Run through the gate. Fix: remove the imports and the crate-wide deny.

### Undocumented unsafe
312 unsafe blocks and 2 unsafe impls have no SAFETY comment, including src/lib.rs:48-49 and 809-811 (`set_len` on an uninitialized string buffer that is then passed as `&mut [u8]`). `parse_str` in src/impls/neon/deser.rs:45 is a safe function taking a raw pointer. Gate count. Fix: add justifications.

### Lint overrides without reasons
86 `#[allow]` attributes without a reason. Gate count. Fix: use `#[expect(..., reason = "...")]`.

### Public types without `Debug`
13 public types, including `Buffers` (src/lib.rs:90) and the iterators in src/value/lazy/object.rs:17-37. Gate count. Fix: derive or implement it.

### Public API can only grow by breaking
src/lib.rs:71, src/prelude.rs:2, src/value.rs:75, src/serde.rs:12-13 are glob re-exports that expose all of `value_trait`. `ErrorType` (src/error.rs:7) is not `#[non_exhaustive]`. Read and scan. Fix: name re-exports and mark the enum.

### Copy-paste documentation errors
src/value/tape/array.rs:45 says "key/value pairs" for an array. src/value/tape/trait_impls.rs:189 says "array" on `as_object`. Read. Fix: correct the wording.

## Note

### Misplaced dependency
Cargo.toml:37. `ref-cast` sits in the x86_64-only target section and `src/` never uses it. Read. Fix: remove it.

### `--all-features` cannot build on stable
Cargo.toml:101. The nightly-only `hints` feature fails with E0554 and the approx/128bit error above also fires, so the skill's default gate needs an explicit feature list. `feature = "portable"` code is unreachable because the feature is commented out at Cargo.toml:110. Run and read.

## Verification

These plain commands assume a nightly with Miri for the Miri lines. Most of them need `RUSTFLAGS="--cap-lints warn"` here.

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --features swar-number-parsing,serde_impl,runtime-detection -- -D warnings`: fail, drift only as far as it got (unused imports).
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items` with the same features: pass.
- `cargo test --locked`, with cap-lints: pass. `--release --lib --tests`: pass.
- `cargo test --lib -- --ignored`: `tests::serde::float2` fails by design. simd-json's value is the more precise one, and the test comment says serde_json is less precise.
- Features tested with `cargo test --lib --tests`: no-default, serde_impl, 128bit, value-no-dup-keys, known-key, approx-number-parsing, arraybackend, ordered-float, beef all pass. big-int-as-float fails to compile its tests.
- `cargo check --features 128bit,approx-number-parsing`: fail. Nine other feature combinations: pass.
- `cargo +nightly-2026-10-01 miri test --lib` on a copy with the deny line removed: stops at the NEON misaligned write.
- The same on a copy with `write_unaligned` patched in, the `with_capacity(0)` test changed, `-Zmiri-tree-borrows`, and proptest and `test::parsing` skipped: 273 passed. Under Stacked Borrows my own probe program fails with the aliasing error above.
- Differential fuzz in throwaway crates, checking accept/reject and value against serde_json and float bits against std. It covered:
  - all inputs up to 4 bytes over a 28-byte alphabet;
  - 1.5M generated numbers;
  - 1.5M mutated documents around 64-byte seams;
  - a rerun with debug assertions and overflow checks on and with round-trip checks across owned, borrowed, tape, lazy, encode and pretty, at 1M each.

  There were no panics. Differences: the float bug, NUL acceptance, and duplicate-key divergence (inferred from the sampled document, not confirmed for all 11 cases). `9e81`, which differed from serde_json in 4 cases, I attribute to serde_json's less precise default float parsing, but did not check against std. The native fallback, on a patched copy forced onto aarch64, showed the same categories.
- `scripts/audit_scan.py`: glob re-exports confirmed. Its TODO notes are not reported separately.

Findings: 47. Last clock reading: 20 minutes used of 45.
