The review found **24 findings: 10 High, 9 Medium, and 5 Low**, using [tooling]’s severity definitions. All 178 existing unit and integration tests passed, but focused regressions demonstrated incorrect output, unexpected panics, and metadata loss. The full test command also encountered the documented memory-heavy doctest failure.

No production source was edited. Harnesses, generated fixtures, and logs are under `/work/tmp`.

## High

### 1. ZIP field-length conversions can abort the process

`crate/src/types.rs:902`, `crate/src/types.rs:938`.

Expected: unrepresentable field lengths return an error.

Observed: a 65,536-byte filename panics at `try_into().unwrap()`. Appending an otherwise valid archive containing a 40,000-byte CP437 comment also panics: decoding expands its UTF-8 representation beyond 65,535 bytes. `finish` then panics again during `Drop`, aborting the process.

Evidence: executed `long_filename_returns_error_without_panic` and the separate `append-long-comment` binary.

validate encoded byte lengths before modifying writer state and use fallible conversions.

### 2. `finish_into_readable` exposes ZIP64 sentinel sizes as actual sizes

`crate/src/write.rs:2078`.

Expected: the returned archive behaves like reopening the written bytes.

Observed: writing five bytes with `large_file(true)` produces a returned entry reporting 4,294,967,295 bytes. Reading it consumes central-directory bytes and fails its checksum. Reopening the same bytes correctly reports five bytes.

Consequence: callers receive incorrect metadata and cannot reliably read their newly written entry.

Evidence: executed `finish_readable_zip64_preserves_sizes_and_contents`.

keep actual sizes in memory; apply sentinel values only during serialization.

### 3. Empty XZ entries produce invalid compressed streams

`crate/src/write.rs:1914`.

Expected: an empty entry round-trips with the supported XZ compression method.

Observed: `finish()` succeeds, but reading the entry fails with `index CRC32 mismatch`. Python’s XZ decoder independently rejects its compressed payload as corrupt. This occurred with locked dependency `lzma-rust2 0.13.0`; nonempty XZ cases passed.

Consequence: ordinary empty files become unreadable archive entries.

Evidence: executed compression matrix and Python decoder comparison; fixture `/work/tmp/fixtures/xz-0.zip`.

correct or accommodate the dependency’s empty-stream finalization behavior and add an empty-entry regression.

### 4. Appending strips required ZIP64 offset information

`crate/src/read.rs:1479`, `crate/src/write.rs:2094`.

Expected: appending preserves existing entries.

Observed: parsing removes the ZIP64 extra field, and central-directory writing does not regenerate it. A valid entry whose local header starts at `0x100000000` becomes unreadable after `new_append(...).finish()`, which returns success.

Consequence: updating a large archive can break existing files.

Evidence: executed `append_preserves_zip64_offset`. The fixture used a sparse prefix and five-byte payload; Python and the original Rust reader both read it successfully before rewriting.

regenerate central-directory ZIP64 fields from actual sizes and offsets.

### 5. Rewriting Implode entries discards required compression flags

`crate/src/types.rs:849`.

Expected: rewriting compressed entries preserves flags needed to decode their unchanged data.

Observed: `flags()` discards the original compression-specific bits. The existing Implode fixture has flags `0x6`; both appending and raw copying rewrite them without those bits. Subsequent reads fail with `too many codeword lengths`.

Consequence: successful archive rewrites corrupt supported legacy entries.

Evidence: executed `append_preserves_implode_compression_flags` and `raw_copy_preserves_implode_compression_flags`; the original fixture reads successfully.

preserve compression-specific flags and update only deliberately changed bits.

### 6. Raw copying ZipCrypto entries loses the encryption flag

`crate/src/read.rs:1832`, `crate/src/types.rs:716`.

Expected: `raw_copy_file` preserves encrypted entry data and metadata.

Observed: copying an untouched `by_index_raw` ZipCrypto entry succeeds, but the destination is marked unencrypted while still containing encrypted bytes. Reading it fails its checksum, including when the correct password is supplied.

Consequence: the copied entry is unusable.

Evidence: executed `raw_copy_zipcrypto_preserves_encryption`; the source decrypts to `hello`.

preserve encryption and descriptor metadata independently of options that request new encryption.

### 7. Raw copying a password-validated AES entry omits its header

`crate/src/write.rs:1336`.

Expected: an accepted `ZipFile` is copied correctly, or an unsupported reader state is rejected.

Observed: `by_index_decrypt` consumes the AES salt and verification bytes during construction. `take_raw_reader()` then copies from after that header. Both copying and finishing succeed, but the destination rejects the correct password.

Consequence: a natural public API composition silently creates an unreadable entry, even before the caller reads any content.

Evidence: executed `raw_copy_aes_from_decrypted_file_preserves_raw_contents`.

retain the complete raw representation or reject this state before writing; document any requirement to use `by_index_raw`.

### 8. Streaming ZipCrypto writes the wrong password-check byte

`crate/src/zipcrypto.rs:168`.

Expected: a streamed encrypted entry accepts its correct password.

Observed: the writer always uses the high CRC byte. Streaming sets the data-descriptor flag, for which the reader correctly expects the DOS-time check byte. Writing and finishing succeed, but reopening returns `InvalidPassword`.

Evidence: executed `zipcrypto_stream_writer_roundtrip` with a fixed timestamp and plaintext `hello`.

select the check-byte convention according to the descriptor flag.

### 9. Streamed files panic when queried for `data_start`

`crate/src/read.rs:1818`, `crate/src/types.rs:827`.

Expected: streaming documentation specifies `data_start` as zero.

Observed: streaming construction leaves the `OnceLock` empty, and `data_start()` unwraps it.

Consequence: querying ordinary file metadata panics.

Evidence: executed `streamed_file_data_start_is_documented_zero`.

initialize the documented sentinel or otherwise handle the unknown offset without panicking.

### 10. Zopfli-only compression panics on unsupported levels

`crate/src/write.rs:1977`, `crate/src/write.rs:1726`.

Configuration: `--no-default-features --features deflate-zopfli`.

Expected: unsupported compression levels return an error, as other out-of-range levels do.

Observed: the accepted range starts at one, but iteration calculation subtracts nine. Levels 1, 6, and 8 panic on subtraction in debug builds; level 9 panics when constructing a nonzero iteration count.

Evidence: executed `/work/tmp/zopfli-probe/tests/levels.rs`; levels 0 and 265 returned errors, while 10 and 264 succeeded.

enforce the documented Zopfli minimum before calculating iterations.

## Medium

### 11. `finish_into_readable` cannot read AES entries

`crate/src/write.rs:785`.

Expected: its returned archive can decrypt the written entries.

Observed: writer metadata retains `CompressionMethod::Aes`, while the reading path expects the underlying compression method. Opening the returned entry fails with `Compression method not supported`. Reopening the same bytes decrypts successfully.

Evidence: executed `finish_readable_aes_can_decrypt_contents`.

normalize writer metadata into the representation expected by readers.

### 12. Raw copying converts symlinks into regular files

`crate/src/read.rs:1836`.

Expected: raw copying preserves the documented file metadata.

Observed: metadata passes through `unix_permissions`, which removes file-type bits, followed by normalization as a regular file. A copied symlink becomes a regular file containing the target text.

Evidence: executed `raw_copy_preserves_symlink_type`.

preserve file kind separately from permission overrides. Source inspection also shows comments and extra metadata are omitted by the options-based copy.

### 13. Stream boundary parsing loses central metadata and rejects empty archives

`crate/src/read.rs:1944`, `crate/src/read/stream.rs:48`.

Expected: visitors receive additional metadata after file contents; an empty archive completes successfully.

Observed: the file reader consumes 30 bytes of the first central-directory record before returning `None`. Metadata parsing then starts at the wrong position, and its error is swallowed. A one-file archive produces zero metadata callbacks. An empty archive fails with `UnexpectedEof`.

Evidence: executed `stream_reader_delivers_central_metadata` and `stream_reader_empty_archive`.

The existing streaming test checks callback contents but never asserts that a callback occurred.

identify record signatures before consuming their bodies, preserve the central-record prefix, and distinguish normal termination from parsing errors.

### 14. Valid Unicode comments are checked against decoded bytes

`crate/src/read.rs:1530`.

Expected: the Unicode comment checksum is validated against the original comment bytes.

Observed: validation uses the already decoded comment’s UTF-8 bytes. A CP437 comment containing byte `0x82` therefore fails validation, and the whole archive is rejected as `Could not find EOCD`.

Evidence: executed `unicode_comment_checksum_uses_original_bytes`; Python reads the fixture, and an ASCII-comment control succeeds.

retain the original comment bytes for checksum validation.

### 15. Appending corrupts CP437 comment text

`crate/src/write.rs:2107`, `crate/src/types.rs:849`.

Expected: unchanged comments retain their text after appending.

Observed: decoded comments are written as UTF-8, but the UTF-8 flag is determined from the filename alone. For ASCII filename `a`, comment `é` becomes `├⌐` after rewriting.

Evidence: executed `append_preserves_cp437_file_comment_text`.

preserve original encoding or consistently convert both names and comments and set the corresponding flag.

### 16. Explicit ZIP64 comments disappear when reopening small archives

`crate/src/spec.rs:365`, `crate/src/spec.rs:638`.

Expected: `set_zip64_comment` round-trips through the public reader.

Observed: the writer emits ZIP64 records, but the reader only considers them when selected classic fields contain sentinel values. A small archive’s `zip64_comment()` consequently returns `None`.

Evidence: executed `zip64_comment_survives_reopening_small_archive`.

recognize a valid ZIP64 locator independently of those sentinels, or emit appropriate sentinel fields when explicitly writing ZIP64 metadata.

### 17. Extraction never restores directory permissions

`crate/src/read.rs:898`.

Expected: directories made temporarily writable during extraction receive their archived modes afterward.

Observed: directory entries bypass the permission-restoration list. A directory archived as `0o500` remains `0o755` after extracting its child.

Evidence: executed `extraction_preserves_directory_permissions` on Linux.

include directories in the existing children-before-parents permission pass.

### 18. Central-directory I/O errors become misleading archive errors

`crate/src/read.rs:660`.

Expected: callers can identify underlying I/O failures through `ZipError::Io`.

Observed: every central-header error is discarded during candidate backtracking. Injecting one I/O error while reading an otherwise valid central header produces `InvalidArchive("Could not find EOCD")`.

Consequence: callers cannot distinguish a read failure from invalid archive data or recover using the original cause.

Evidence: executed `central_directory_io_error_is_preserved`.

propagate I/O errors and limit backtracking to appropriate structural failures.

### 19. Strict documentation builds fail on five broken links

Locations: `crate/src/read/magic_finder.rs:218`, `crate/src/read.rs:1617`, `crate/src/spec.rs:95`, `crate/src/spec.rs:143`.

Expected: the required documentation command succeeds.

Observed: five unresolved links involving `Self::next_back`, `PathBuf::Component::ParentDir`, and `ZipWriter` fail rustdoc with warnings denied.

Evidence: executed all-features documentation gate on Rust 1.99.1.

correct the qualified paths and remove or replace the invalid constant-associated link.

## Low

### 20. Strict Clippy fails on the installed toolchain

`crate/src/write.rs:2173`.

The all-targets, all-features Clippy gate fails on `clippy::collapsible_match`. This is pre-existing; behavior on Rust 1.83 was not established.

express the condition as a match guard.

### 21. Public failure contracts lack documentation

Example location: `crate/src/write.rs:1187`.

The advisory pass reports 59 missing `# Errors` sections, including archive access and writing APIs. Callers must inspect implementation details to learn feature restrictions and failure behavior.

Evidence: executed advisory lint pass and source inspection.

document meaningful failure conditions, prioritizing the APIs implicated above.

### 22. Three public reader/writer types lack `Debug`

Locations: `crate/src/read.rs:246`, `crate/src/read.rs:252`, `crate/src/write.rs:2136`.

`ZipFile`, `ZipFileSeek`, and `StreamWriter` lack `Debug`, limiting diagnostics and preventing containing caller types from deriving it.

Evidence: executed advisory pass and source inspection.

provide suitable manual or derived implementations.

### 23. Ten unsafe sites lack local safety justifications

Example locations: `crate/src/spec.rs:157`, `crate/src/types.rs:1004`.

Three unsafe blocks and seven unsafe implementations lack local `SAFETY` comments. The inspected layouts use packed integer fields, but future layout changes require maintainers to reconstruct why byte casting remains valid.

Evidence: advisory pass and source inspection. No undefined-behavior finding was established.

document each implementation’s layout and valid-bit-pattern requirements.

### 24. Lint suppressions do not expose stale exceptions

Example locations: `crate/src/compression.rs:63`, `crate/src/lib.rs:34`.

The advisory pass reports 35 `allow` attributes without a `reason` argument; several have explanatory nearby comments. Persistent suppressions can remain unnoticed after their original need disappears.

record reasons and use `expect` where an exception should be checked for continued applicability.

## Verification and artifacts

Cargo commands used `RUSTUP_AUTO_INSTALL=0` and offline dependency resolution.

From `/work/crate`:

- `cargo test --locked --all-features`: **178 unit/integration tests passed**; nine doctests passed; `spec::ZIP64_BYTES_THR` was killed with SIGKILL. This matches the stated allocation constraint and is not counted as a defect.
- `cargo fmt --all --check`: **passed**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: **failed**, finding 20.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: **failed**, finding 19.
- `cargo test --locked --all-features --doc -- --skip spec::ZIP64_BYTES_THR`: **nine passed**.
- `cargo test --locked --no-default-features --lib --tests`: **106 passed**.
- `cargo test --locked --no-default-features --features deflate-zopfli --lib --tests`: **107 passed**.
- `cargo check --locked --no-default-features --features _deflate-any`: **failed** with missing variables and a type mismatch. This internal switch alone is not counted as a supported-configuration finding.

The requested audit script and heuristic scanner ran. Their outputs are in `/work/tmp/gate.log` and `/work/tmp/scan.log`.

Scratch verification:

- `/work/tmp/review-harness`: `cargo test --test behavior -- --nocapture --test-threads=1` produced **20 failing regression assertions and three passing controls** across 23 scenarios. Detailed output: `/work/tmp/behavior-final.log`.
- Streaming AES’s seek failure is covered by `new_stream`’s stated seeking restriction and is **not counted separately**.
- `/work/tmp/zopfli-probe`: `cargo test -- --nocapture` confirmed finding 10.
- `cargo run --bin append-long-comment` confirmed the destructor panic and process abort.
- Both scratch packages passed formatting and strict Clippy.
- Python validated the generated archive controls and independently rejected the empty XZ payload.

Miri:

- `cargo +nightly-2026-10-01 miri test --locked --no-default-features --lib spec::test`: **one test passed**.
- Broader library runs first encountered filesystem isolation restrictions. With isolation disabled, attempts ended with exit 137 at the 64K-entry test and, after skipping it, at `test_central_directory_not_at_end`. These are resource/time blocks, not defect evidence.

Finding count: **24 — 10 High, 9 Medium, 5 Low**. Coverage was aarch64 Linux with the stated toolchains; Rust 1.83, other platforms, exhaustive feature combinations, and complete Miri coverage remain unverified. No dedicated security testing was performed.

Last clock reading: **18.8347 of 45 minutes used; 1,133.7 seconds active time**.