The release passes its existing unit and integration tests in the configurations checked, but public-API regression probes found substantial correctness gaps in copying, ZIP64, streaming, extraction, and metadata handling. Findings: 2 Blocker, 18 Major, and 5 Minor. Resource-limit terminations are reported separately and are not treated as defects.

The original crate was not edited. Reproducers and generated fixtures are retained in the [scratch harness](/work/tmp/zip-review/Cargo.toml). Test names below refer to that harness unless stated otherwise.

## Blocker

### Streaming metadata accessor panics on valid input

Location: [src/read.rs:1818](/work/crate/src/read.rs:1818).

`read_zipfile_from_stream()` documents `data_start` as zero, but leaves its cache unset; calling `data_start()` unwraps `None`. Ordinary metadata inspection therefore panics on a valid streamed entry.

Executed evidence: `streaming_data_start_matches_documented_zero` panicked at this line. Fix: initialize or return the documented streaming value.

### Oversized filenames panic instead of returning an error

Location: [src/types.rs:902](/work/crate/src/types.rs:902).

A 65,536-byte filename reaches an unchecked conversion to `u16` inside `start_file()`. The API returns `Result`, but this invalid input panics, preventing ordinary error recovery.

Executed evidence: `too_long_filename_returns_error` caught the panic; the neighboring 65,535-byte case passed. Fix: validate encoded name length before inserting or writing the entry.

## Major

### Raw copying loses ZipCrypto encryption state

Location: [src/read.rs:1833](/work/crate/src/read.rs:1833), [src/write.rs:1316](/work/crate/src/write.rs:1316).

An untouched raw copy should preserve encryption metadata. Instead, `file.options()` loses the ZipCrypto encrypted flag while ciphertext is copied unchanged. Readers consequently treat the copied bytes as plaintext.

Executed evidence: `raw_copy_zipcrypto_retains_encryption` observed `encrypted() == false` after reopening. Source inspection confirms that password handling then discards the supplied password. Fix: preserve raw encryption flags independently of options that request new encryption.

### Raw copying accepts streams whose original bytes are unavailable

Location: [src/write.rs:1336](/work/crate/src/write.rs:1336).

The copy operation retains the original sizes and checksum but copies only the remaining underlying bytes. Reading one byte beforehand can also discard unread buffered bytes when extracting the underlying reader. Opening an AES entry for decryption already consumes its encryption header.

Executed evidence: raw copying returned success, but `copy_before_and_after_read_reports_errors_instead_of_damaged_output` subsequently failed checksum validation; `raw_copy_decrypted_aes_entry` failed with `InvalidPassword`. Fix: enforce an untouched raw-reader state before writing, reject unsupported states, and verify the copied length.

### Raw copying discards promised metadata

Location: [src/write.rs:1316](/work/crate/src/write.rs:1316), [src/read.rs:1836](/work/crate/src/read.rs:1836).

The documented operation copies metadata, but reconstruction through `SimpleFileOptions` drops comments, extra fields, and special-file mode bits. A copied symlink becomes a regular file containing its target text.

Executed evidence: `raw_copy_preserves_symlink_type`, `raw_copy_preserves_extra_fields`, and `raw_copy_preserves_comment` failed. Fix: preserve source metadata directly, changing only fields explicitly requested by the copy operation.

### `finish_into_readable()` cannot read newly written AES entries

Location: [src/write.rs:785](/work/crate/src/write.rs:785).

The returned archive retains the writer’s AES wire-method marker instead of the underlying compression method expected by the reader. Correct-password access fails with `UnsupportedArchive("Compression method not supported")`.

Executed evidence: `finish_into_readable_preserves_header_compression_method_aes` failed. Existing AES round-trip tests using `finish()` followed by `ZipArchive::new()` passed. Fix: normalize writer metadata into the reader representation before constructing the archive.

### `finish_into_readable()` retains ZIP64 sentinel sizes

Location: [src/write.rs:2078](/work/crate/src/write.rs:2078).

Updating a ZIP64 local header replaces the actual in-memory sizes with `u32::MAX`. Reusing that metadata reports a five-byte entry as 4,294,967,295 bytes and lets a stored reader consume central-directory bytes.

Executed evidence: `finish_into_readable_preserves_zip64_sizes` reported both sentinel sizes; `finish_into_readable_zip64_reads_only_file_content` read 97 bytes and failed its checksum. Fix: retain actual sizes in metadata and apply sentinels only during serialization.

### Copying and merging fail to rebuild ZIP64 offset fields

Location: [src/read.rs:596](/work/crate/src/read.rs:596), [src/write.rs:700](/work/crate/src/write.rs:700).

Moving entries beyond the ZIP32 offset limit updates `header_start` without rebuilding the corresponding ZIP64 extra data. Merged entries can point to an invalid header; deep copies can still point to the original entry’s header.

Executed evidence: sparse-file tests at offset 4,294,967,396 reproduced both failures. Python’s ZIP reader rejected the merge with “Bad magic number” and the deep copy because its central and local filenames differed. The ordinary large-offset write control passed.

Fix: generate ZIP64 offset fields from the final metadata during serialization.

### Streaming ZIP64 rejects or misreports legitimate large files

Location: [src/write.rs:2053](/work/crate/src/write.rs:2053), [src/write.rs:920](/work/crate/src/write.rs:920).

With `large_file(true)`, finishing a stored 4 GiB entry incorrectly returns “Large file option has not been set.” A deflated 4 GiB entry finishes successfully but reopens with an uncompressed size of zero because its central ZIP64 sizes remain placeholders.

Executed evidence: both streaming tests in `large_stream.rs` failed; the seekable stored control passed. Tests used a bounded-memory sparse sink.

Fix: correct descriptor-size validation and serialize actual ZIP64 sizes into the central directory.

### Empty XZ entries contain an invalid compressed stream

Location: [src/write.rs:1837](/work/crate/src/write.rs:1837).

An empty entry written with XZ should decompress to zero bytes. With the locked `lzma-rust2 0.13.0`, the generated stream fails with “index CRC32 mismatch.”

Executed evidence: `empty_xz_cross_check` and both empty-XZ matrix cases failed. Python’s independent `lzma` decoder also rejected the generated 40-byte stream as corrupt; nonempty XZ cases passed.

Fix: add an empty-entry regression and correct or replace the backend’s empty-stream handling. The defect was reproduced through the crate API; localization within the dependency was not completed.

### Extraction rejects ordinary links within the destination

Location: [src/read.rs:1721](/work/crate/src/read.rs:1721).

The extraction contract permits following links within the destination. However, this code passes the canonical absolute destination to a helper that rejects absolute paths, producing `InvalidArchive("Invalid base path")`.

Executed evidence: extraction of `real/`, `link -> real`, and `link/file` failed. Extracting the same archive containing a symlink twice also failed, contrary to the overwrite contract.

Fix: distinguish relative-path normalization from absolute-base comparison and add these ordinary extraction cases.

### Valid Unicode comments reject an entire archive

Location: [src/read.rs:1530](/work/crate/src/read.rs:1530).

The Unicode-comment checksum must cover the original comment bytes. This code checks the already-decoded UTF-8 comment instead. Valid non-ASCII CP437 comments therefore fail validation and prevent opening any entry.

Executed evidence: `unicode_comment_uses_crc_of_original_bytes` failed with “Could not find EOCD”; the ASCII control passed. Fix: retain the original comment bytes for checksum validation before decoding.

### Stale optional Unicode paths reject otherwise readable archives

Location: [src/read.rs:1539](/work/crate/src/read.rs:1539).

Under the referenced Unicode-path format contract, a checksum mismatch means the optional Unicode name should be ignored in favor of the primary filename. Here it invalidates the entire archive.

Executed evidence: `stale_unicode_path_uses_current_name` failed to open an archive whose primary name was `file` and whose optional field retained an older name. Python’s ZIP reader read `file` successfully. Fix: fall back to the primary name when optional Unicode metadata fails validation.

### Streaming ZipCrypto output rejects the correct password

Location: [src/zipcrypto.rs:168](/work/crate/src/zipcrypto.rs:168), [src/read.rs:412](/work/crate/src/read.rs:412).

The writer always uses the CRC verification byte. Streaming entries set the data-descriptor flag, for which the reader correctly expects the modification-time verification byte.

Executed evidence: `stream_zipcrypto_roundtrip` finished successfully but returned `InvalidPassword` on reopening with the same password. Fix: select the verification convention according to the entry’s descriptor flag. This concerns the unstable ZipCrypto API.

### Appending rewrites AES central-directory methods incorrectly

Location: [src/types.rs:920](/work/crate/src/types.rs:920).

AES entries require wire compression method 99, with their actual compression method in the AES extra field. After reading, this serializer writes the underlying method instead.

Executed evidence: `append_preserves_aes_wire_method` observed the final central-directory method change from 99 to 0 after opening and finishing an append writer without adding files. The local header remained AES, creating inconsistent headers for other readers.

Fix: serialize method 99 whenever AES metadata is present.

### Stream visitors silently receive no additional metadata

Location: [src/read.rs:1944](/work/crate/src/read.rs:1944), [src/read/stream.rs:48](/work/crate/src/read/stream.rs:48).

The local-entry reader consumes 30 bytes before recognizing the central-directory signature. Central-directory parsing then starts midway through its first record, and the visitor silently suppresses the resulting error.

Executed evidence: `stream_visitor_gets_all_metadata` received two file callbacks and zero metadata callbacks while returning success. Existing tests only validate metadata callbacks that happen to occur, making their checks ineffective here.

Fix: preserve sufficient lookahead for the central parser and distinguish the expected footer from parsing failures.

### ZIP64 comments disappear on small archives

Location: [src/spec.rs:365](/work/crate/src/spec.rs:365), [src/write.rs:1569](/work/crate/src/write.rs:1569).

Setting a ZIP64 comment writes a ZIP64 footer, but the reader only searches for that footer when selected ZIP32 fields contain sentinels. Small archives therefore lose access to the comment.

Executed evidence: `zip64_comment_roundtrip` set `"hello"` and observed `None` after reopening. Fix: detect a valid ZIP64 locator without requiring overflow sentinels, or emit appropriate sentinels when forcing ZIP64.

### Extraction never restores directory permissions

Location: [src/read.rs:900](/work/crate/src/read.rs:900).

Directories are made temporarily writable, then skipped before their stored modes enter the restoration queue. Successful extraction consequently leaves different permissions from those recorded in the archive.

Executed evidence: `extraction_preserves_explicit_directory_permissions` observed `0755` for a directory stored as `0555`. Fix: queue directory modes before continuing and restore them after their children.

### Access-only timestamps are reported as modification times

Location: [src/extra_fields/extended_timestamp.rs:51](/work/crate/src/extra_fields/extended_timestamp.rs:51).

A valid five-byte local timestamp field with only the access-time flag should populate `ac_time()`. The unconditional five-byte heuristic instead populates `mod_time()`.

Executed evidence: `local_access_timestamp_is_not_modification_timestamp` returned `(Some(123456789), None, None)` instead of `(None, Some(123456789), None)`.

Fix: distinguish local and central timestamp-field rules.

### Strict documentation generation fails

Locations: [src/read/magic_finder.rs:218](/work/crate/src/read/magic_finder.rs:218), [src/read.rs:1617](/work/crate/src/read.rs:1617), [src/spec.rs:95](/work/crate/src/spec.rs:95), [src/spec.rs:143](/work/crate/src/spec.rs:143).

The required strict rustdoc command fails on five unresolved links: `Self::next_back`, `PathBuf::Component::ParentDir`, and three `ZipWriter` references. This prevents passing the documentation gate with the supplied toolchain.

Executed evidence: the audit script’s rustdoc step failed. Fix: correct and qualify the linked paths.

## Minor

### Documented compression level zero is rejected

Location: [src/write.rs:445](/work/crate/src/write.rs:445), [src/write.rs:1975](/work/crate/src/write.rs:1975).

The documentation permits Deflate level zero, but validation starts at `Compression::fast()`, which is one.

Executed evidence: `invalid_compression_levels_match_documented_zero_boundary` rejected Deflate level zero with `UnsupportedArchive`. Callers following the documented range receive an unexpected setup error. Fix: accept supported zero-level compression or correct the documented range and test its endpoints.

### Clippy fails on the supplied newer toolchain

Location: [src/write.rs:2173](/work/crate/src/write.rs:2173).

Rust 1.99.1 Clippy rejects the nested condition with `clippy::collapsible_match` under `-D warnings`. Allowing only that lint made the remaining strict Clippy check pass.

Fix: use a guarded match arm. This is a toolchain-dependent lint failure; no comparison with Rust 1.83 was possible.

### Unsafe representation assumptions lack local justification

Locations: [src/spec.rs:157](/work/crate/src/spec.rs:157), [src/spec.rs:260](/work/crate/src/spec.rs:260), [src/types.rs:1004](/work/crate/src/types.rs:1004).

The advisory pass reported three unsafe blocks and seven unsafe implementations without safety comments. Inspection found packed integer representations underlying these operations, but the assumptions connecting those layouts to byte casting are not recorded beside them.

This makes future layout changes harder to review correctly. Fix: document the initialization, padding, and valid-bit-pattern arguments at each site. No undefined behavior was established in these production operations.

### Three public types lack `Debug`

Locations: [src/read.rs:246](/work/crate/src/read.rs:246), [src/read.rs:252](/work/crate/src/read.rs:252), [src/write.rs:2136](/work/crate/src/write.rs:2136).

`ZipFile`, `ZipFileSeek`, and `StreamWriter` lack `Debug`. This prevents convenient diagnostics and methods such as `unwrap_err()` on results whose success value contains these types.

Evidence: source inspection and three advisory compiler warnings. Fix: provide bounded or manual implementations that avoid dumping underlying buffers.

### The non-UTF-8 writer test constructs invalid `String` values

Location: [src/write.rs:2401](/work/crate/src/write.rs:2401), [src/write.rs:2407](/work/crate/src/write.rs:2407).

`write_non_utf8` passes GB18030 and Shift-JIS bytes to `String::from_utf8_unchecked`, violating that function’s UTF-8 requirement. It therefore does not establish behavior available to ordinary safe callers and can become unreliable when string operations change.

Evidence: source inspection. Native and targeted Miri executions passed; neither result validates the violated precondition. Fix: construct byte fixtures directly or exercise a valid public path/input API.

## Verification

All Cargo commands used `RUSTUP_AUTO_INSTALL=0` and offline dependency access. Host: aarch64 Linux; Rust/Cargo 1.99.1; installed Miri toolchain `nightly-2026-10-01`.

From `/work/crate`:

- `cargo test --locked --all-features`: 136 unit and 42 integration tests passed. Nine doctests passed; the known approximately 4 GiB ZIP64 doctest was SIGKILLed. Overall exit 101.
- `cargo test --locked --lib --tests`: 165 passed.
- `cargo test --locked --no-default-features --lib --tests`: 106 passed.
- `cargo test --locked --no-default-features --features deflate-zopfli --lib --tests`: 107 passed.
- `cargo test --locked --all-features --doc -- --skip spec::ZIP64_BYTES_THR`: nine passed, one filtered.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: failed on `collapsible_match`. The locked rerun with only `-A clippy::collapsible_match` passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: failed on five broken links.
- `bash /work/skill/scripts/check.sh --audit /work/crate`: completed; format passed, Clippy/rustdoc/tests failed as described above.
- `python3 -I /work/skill/scripts/audit_scan.py /work/crate`: completed. Four glob re-export and eight TODO candidates were inspected and not promoted without a concrete consequence.

The advisory pass produced 187 warnings, including 59 missing `# Errors` sections, five missing `# Panics` sections, and 35 allow attributes without structured reasons. These were not counted mechanically as separate defects.

Miri:

- `cargo +nightly-2026-10-01 miri test --locked --no-default-features --lib spec::test::block_serde`: passed.
- The equivalent targeted `write::test::write_non_utf8` run passed.
- The complete no-default-feature library run stopped at unsupported filesystem access under isolation. Retrying with `MIRIFLAGS=-Zmiri-disable-isolation` reached `test_64k_files` before exit 137. Full Miri coverage remains blocked by resources.

From `/work/tmp/zip-review`, `cargo test --locked --no-fail-fast` executed 38 tests: eight controls passed and 30 assertions failed. The [complete output](/work/tmp/review-tests-final.log) is retained. Harness formatting and strict Clippy passed. Two exploratory assertions were not promoted: seeking past EOF is explicitly clamped by this implementation, and AES streaming encounters the constructor’s documented restriction on backward writes.

Total: 25 findings—2 Blocker, 18 Major, 5 Minor. Coverage excludes dedicated security testing, exhaustive feature combinations, Windows/macOS execution, and Rust 1.83 verification; those targets/toolchain were unavailable, and `cargo-hack` was not installed. Last clock reading: 1,288.3 seconds active, 21.47 of 45 minutes.