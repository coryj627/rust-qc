# Audit report: zip 5.1.1

Verdict: the crate builds and its own tests pass, but it has serious defects outside what those tests cover. Extraction has security gaps. Crafted archives can panic or abort the process. `new_append`, `merge_archive`, `raw_copy_file` and `deep_copy_file` can corrupt archives. `ZipWriter::new_stream` and `ZipStreamReader` are broken in several ways.

Findings per severity: 15 Blocker, 28 Major, 9 Minor, 2 Note, 54 in total.

All probes ran through `./run` from a throwaway crate at `probe/`, and `crate/` source was not changed. "Debug" below means overflow checks on. Other tools means `unzip` 6 and Python `zipfile` in the container. Nothing in the repository addressed me.

## Blocker

### Extraction applies setuid and setgid bits from the archive
Location: src/read.rs:942 (mode comes from src/types.rs:622).
Problem: `ZipArchive::extract` passes the whole archive mode to `chmod`. An entry with mode 0o106755 is extracted with setuid and setgid set, so extracting as root can plant a setuid binary.
Established: ran it; the on-disk mode was 106755.
Fix: mask with 0o777, and only keep higher bits on explicit request.

### Symlinks pointing outside the destination are created, contrary to the docs
Location: src/read.rs:908 (docs at 768-770).
Problem: the docs say links are "only created and followed if the target is within the destination directory". No target check happens before `make_symlink`, so anything that later follows the link leaves the destination.
Established: ran it; `evil -> /etc/passwd` and `evil2 -> ../../../../etc/hostname` were both created and `extract` returned Ok.
Fix: validate the target before creating the link, or correct the docs and add an opt-in.

### AES authentication is skipped on empty or short data
Location: src/aes.rs:171 and src/aes.rs:178.
Problem: when `data_remaining` is 0 at the start, or the inner read returns 0 early, `read` returns Ok(0) without checking the HMAC. AE-2 also has no CRC, so tampering goes undetected.
Established: ran both cases with the correct password.
- Patching the central `compressed_size` to header-only returned empty content and Ok, although the declared size was 5.
- Patching it past EOF returned 95 bytes of garbage and Ok.
- A control that flipped a ciphertext bit was correctly rejected.
Fix: always verify the auth code at end of data, and treat an early EOF as an error.

### Symlink entry with a huge declared size panics or aborts extract
Location: src/read.rs:894 (the same pattern is at src/read/stream.rs:71, currently unreachable).
Problem: `Vec::with_capacity(file.size() as usize)` uses the attacker's Zip64 size.
Established: ran it, in debug and release. Size `u64::MAX` panics with "capacity overflow". Size 2^62 aborts the process with "memory allocation of 4611686018427387904 bytes failed".
Fix: do not preallocate from the declared size, and cap symlink targets.

### Shrink stream overflows the stack
Location: src/legacy/shrink.rs:141.
Problem: `read_code` recurses once per control code with no depth limit. This is an uncatchable abort with the `legacy-zip` feature.
Established: ran it, in debug and release. A 45 KB payload (20,000 partial-clear pairs) was fine. 200,000 pairs (about 450 KB) gave "has overflowed its stack" and abort.
Fix: use a loop.

### Legacy decoders panic on a large declared size
Location: src/legacy/implode.rs:97 and 214, src/legacy/reduce.rs:107, src/legacy/shrink.rs:227 and 340.
Problem: `reserve(uncompressed_size as usize)` trusts the header, so reading a crafted entry panics. Fuzzing also showed the panic in the stream path.
Established: ran it; Zip64 `u64::MAX` gave "capacity overflow" for Shrink, Reduce and Implode, in debug and release.
Fix: use `try_reserve` with a cap, or grow as output is produced.

### Xz entry with a crafted index aborts the process
Location: src/compression.rs:444 (the allocation is in lzma-rust2 0.13).
Problem: untrusted Xz data goes to a dependency that allocates from the stream's record count. Xz is a default feature.
Established: ran it; a 13-byte stream header plus a record count of 2^56 gave "memory allocation of 1152921504606846976 bytes failed" and abort. Fuzzing also hit "capacity overflow".
Fix: update or patch the dependency, or isolate it behind a limit.

### `new_append` then `finish` aborts on a long non-ASCII entry comment
Location: src/types.rs:938.
Problem: the comment is decoded from CP437 to UTF-8, which can expand past 65535 bytes, and `try_into().unwrap()` panics. `Drop` then panics again, so the process aborts.
Established: ran it; a 30,000-byte comment of 0xB0 gave `TryFromIntError`, then "panic in a destructor during cleanup", exit 134.
Fix: keep the raw comment bytes and return an error. Do not re-enter finalisation after a failure.

### A file name over 65535 bytes aborts the process
Location: src/types.rs:902 and 934.
Problem: `start_file` panics on `unwrap`, then `Drop` runs `block()` and panics again.
Established: ran it; a 70,000-character name gave panics at 902 and 934, then abort.
Fix: reject the name with `Err` before adding it to `files`.

### `new_append` and `merge_archive` lose Zip64 data
Location: src/write.rs:2094; the field is stripped at src/read.rs:1433-1441.
Problem: reading removes the Zip64 extra field, and the central directory writer never regenerates it. Any existing entry that needs Zip64 comes back wrong or unreadable. `merge_archive` past 4 GiB has the same problem.
Established: ran it on a real 4.36 GB sparse archive.
- After `new_append`, "big" reported size 4294967295, and "after" (offset 4362076213) failed with "Invalid local file header".
- An entry merged past 4 GiB failed the same way.
Fix: emit a Zip64 field in the central header whenever a value reaches the threshold.

### `new_append` makes archives with Info-ZIP Unicode paths unreadable by this crate
Location: src/read.rs:1538-1542, written at src/write.rs:2098-2101.
Problem: the raw name is replaced by the UTF-8 name but the 0x7075 field is kept, so its CRC no longer matches on the next open.
Established: ran it; the name was "café.txt" before, and reopening gave `InvalidArchive("Could not find EOCD")`. Python still opens the file.
Fix: keep the original raw name, or drop the extra field when rewriting.

### `new_append` clears general-purpose flag bits on existing entries
Location: src/types.rs:849-865.
Problem: `flags()` keeps only three bits, so Implode's dictionary and tree bits are lost.
Established: ran it on tests/data/legacy/implode.zip; flags went from 6 to 0, and `first.txt` then failed with "too many codeword lengths". Before the append it read 1092 bytes.
Fix: preserve the parsed flags and set only the bits being changed.

### `raw_copy_file` of a large entry writes sizes of 0
Location: src/write.rs:920 and 1088.
Problem: the Zip64 field is built with zero sizes, and the raw path skips the later update. The central size fields then point to the zeroed Zip64 values.
Established: ran it; the source size was 5368709120, and after copying `size()` was 0.
Fix: build the field from `raw_values`.

### `raw_copy_file` of a ZipCrypto entry clears the encrypted flag
Location: src/types.rs:716 (options built at src/read.rs:1832).
Problem: `options()` carries no ZipCrypto state, so ciphertext is marked as plaintext and can no longer be decrypted.
Established: ran it; the flag was false after the copy. Both a plain read and `by_index_decrypt` failed with "Invalid checksum" after 19 bytes. The AES control worked.
Fix: carry the encrypted flag through the raw path.

### `large_file(true)` overwrites sizes with 0xFFFFFFFF, breaking later use
Location: src/write.rs:2078-2079; the deep-copy clamp is at src/write.rs:684-687.
Problem: in-memory metadata is replaced by the marker value, with two effects.
- `finish_into_readable` reads past the entry.
- `deep_copy_file` copies following entries too, and rewrites the source's size.
Established: ran both.
- A 5-byte entry reported size 4294967295 and failed with "Invalid checksum".
- With entries a, b and a deep copy of a to c, both a and c failed. Reopening with `ZipArchive::new` was fine.
- A 600,000-iteration writer model test failed on this deep-copy path. A 200,000-iteration rerun without `large_file` passed.
Fix: write markers to disk only and keep the true sizes in memory.

## Major

### `ZipStreamReader` never reaches the metadata stage
Location: src/read.rs:1944-1948 and src/read/stream.rs:48.
Problem: the 30 bytes read to detect the central directory are consumed, so the next parse sees the wrong magic and the `while let Ok` ends silently. Metadata is never visited. As a result, `extract` loses permissions and writes symlinks as regular files.
Established: ran it; 2 files gave 0 metadata visits, a 750 file became 644, and a symlink became a regular file.
Fix: pass the already-read block to the central header parser, and report parse errors.

### A test passes for the wrong reason, hiding stream bugs
Location: src/read/stream.rs:383.
Problem: `writer.finish()` returns a cursor at the end, so `extract` fails with EOF rather than on the symlink check. The other stream tests' metadata callbacks never run either.
Established: ran the same pattern; position equalled length (102) and the error was `UnexpectedEof`.
Fix: rewind the cursor, assert the specific error, and assert the metadata count.

### Stream reader rejects a valid empty archive
Location: src/read.rs:1944.
Problem: it always reads 30 bytes, but an empty zip is 22 bytes.
Established: ran it; `ZipArchive` reports length 0, the stream reader returns `UnexpectedEof`.
Fix: read the 4-byte magic first and treat EOCD as the end.

### Stream reader cannot read data-descriptor entries, including this crate's own output
Location: src/types.rs:782-786.
Problem: common streamed zips are rejected, and neither `read_zipfile_from_stream` nor `ZipStreamReader` documents it.
Established: ran it on `new_stream` output; the error was "The file length is not available in the local header".
Fix: support descriptors, at least for Stored with a signature scan and for self-terminating methods, or document the limit.

### `ZipFile::data_start()` panics on stream entries
Location: src/read.rs:1818 (docs at 1935 say "set to 0").
Established: ran it; "called `Option::unwrap()` on a `None` value".
Fix: return 0 as documented, or return `Option<u64>`.

### Unicode extra fields can reject a whole archive
Location: src/read.rs:1530 and 1538-1541, src/extra_fields/zipinfo_utf8.rs:20.
Problem: there are three cases.
- A stale CRC is an error, but APPNOTE says to ignore the field.
- Invalid UTF-8 content is an error.
- The comment CRC is computed over the decoded comment rather than the raw bytes, so a valid field on a CP437 comment fails.
Established: ran all three; each gave "Could not find EOCD". An ASCII-comment control passed. `unzip` and Python accepted the stale-CRC and CP437-comment files, the two I tried with them.
Fix: pass the raw bytes, and ignore invalid fields.

### NTFS and extended timestamp variants reject a whole archive
Location: src/extra_fields/ntfs.rs:33-52, src/extra_fields/extended_timestamp.rs:43.
Problem: these fields are optional metadata, yet NTFS with an extra attribute (length 36) or reserved bytes only (length 4), and UT with length 9 and flags 1, fail the whole file.
Established: ran it; all were rejected. `unzip` and Python opened the length-36 and UT files.
Fix: skip unknown attributes and tolerate mismatches.

### Real errors are replaced by "Could not find EOCD"
Location: src/read.rs:660-666.
Problem: every error from central directory parsing is discarded, including I/O failures and multi-disk. Callers cannot tell a disk error from a bad file.
Established: ran it; a reader returning `PermissionDenied` during the central directory, and a multi-disk EOCD, both gave `InvalidArchive("Could not find EOCD")`.
Fix: keep the first error and return it if no candidate succeeds. Propagate I/O errors immediately.

### EOCD retries give quadratic work
Location: src/read.rs:650-666, src/spec.rs:619.
Problem: each fake EOCD triggers a re-parse of the central directory.
Established: ran it; 69 KB took 0.12 s, 138 KB 0.42 s, 276 KB 1.17 s and 552 KB 4.83 s, about 4x per doubling.
Fix: limit candidates, and stop on a candidate that parsed but had a bad directory.

### `has_overlapping_files` is quadratic
Location: src/read.rs:997.
Established: ran it on legitimate archives; 40k entries took 0.23 s, 80k 0.89 s and 160k 3.53 s.
Fix: sort the ranges and scan.

### Following any symlink always fails with "Invalid base path"
Location: src/read.rs:1718-1723.
Problem: `simplified_components` returns None for an absolute base path, so the check always errors and the `is_absolute_enclosed` code is dead. The docs promise that safe links are followed and that files are overwritten.
Established: ran three cases.
- Extracting the same archive twice with a symlink failed on the second run.
- A file through an inner relative link failed with "Invalid base path".
- An absolute link inside the destination failed with "Invalid symlink target path".
Fix: resolve the target relative to the link's directory and compare it to the base.

### Unix-made entries with zero high attribute bits extract as mode 000
Location: src/types.rs:622.
Problem: `external_attributes` 0x20 with system Unix gives `unix_mode() == Some(0)`, and the file becomes inaccessible. `unzip` has code that treats a zero mode as absent, from my memory of its source; I did not run this case.
Established: ran it; the on-disk mode was 0.
Fix: return None when the Unix bits are 0.

### Directory modes are ignored, and the extraction privacy claim is false
Location: src/read.rs:898-900 and 315-329, docs at 773-775.
Problem: directories never get their archive mode. The docs also say partly extracted new directories are not usable by other users, but they use the umask default.
Established: ran it; 0700 and 0555 directories both became 755. After a failed extract, the new directories were 755.
Fix: create directories 0700 and apply final modes deepest-first, or correct the docs.

### Correct ZipCrypto password rejected when the date is invalid
Location: src/read.rs:414.
Problem: the check byte needs the raw time, but `last_modified_time` is None for an invalid date, so 0 is used.
Established: ran it; a hand-encrypted entry decrypted with a valid date and gave `InvalidPassword` with date field 0.
Fix: keep the raw MS-DOS time.

### `new_stream` with ZipCrypto produces entries nobody can open
Location: src/zipcrypto.rs:168 and src/write.rs:1009.
Problem: the check byte is taken from the CRC while bit 3 is set, and readers then expect the time byte.
Established: ran it, across six compression methods. This crate gave "provided password is incorrect", Python gave "Bad password" and `unzip` skipped the file. The non-streaming output decrypted fine.
Fix: use the time byte when writing a data descriptor.

### `new_stream` with AES always fails at finish
Location: src/write.rs:1117 and 2019.
Problem: the AES field update seeks backward. `start_file` and writes succeed, then all data is lost at the end. The error text is only "i/o error".
Established: ran it across all compression methods tested.
Fix: write the final AES field up front, or reject this at `start_file`.

### `new_stream` with `large_file` over 4 GiB fails at finish
Location: src/write.rs:2053.
Problem: this branch only runs for large files, yet it errors with "Large file option has not been set". The check looks inverted.
Established: ran it; 4311744512 bytes to a sink, and finish returned an error.
Fix: remove the check, and error on the non-large path instead.

### `new_stream` continues after a failed `abort_file` and corrupts an earlier entry
Location: src/write.rs:1175-1179.
Problem: the entry is popped, then the seek fails before `writing_to_file` is reset, so the next `finish_file` overwrites the previous file's sizes and CRC.
Established: ran it; a, b, abort, c gave "a" failing with "Invalid checksum" after 12 bytes.
Fix: check seekability first, and leave state unchanged on error.

### `new_stream` sets bit 3 without writing a descriptor
Location: src/write.rs:1009, with the missing writes at 1451 (directories) and 1088 (raw copy).
Established: ran it; `unzip -t` reports "invalid zip file with overlapped components (possible zip bomb)" for both the directory and raw-copy cases. Plain files were fine, and Python accepted the archives.
Fix: write the descriptor or clear the bit.

### Raw copies lose file type and metadata
Location: src/read.rs:1836, src/types.rs:738 and 849-865.
Problem: the docs say metadata is copied, but several things change.
- Symlinks become regular files (mode 100777, `is_symlink` false).
- Directories become 100755.
- Comments and extra fields are dropped.
- Implode flags go from 6 to 0.
Established: ran it.
Fix: copy type bits, comment, extras and flags.

### `start_file` with options from a parsed AES entry stores plaintext marked as encrypted
Location: src/read.rs:1845-1849 and src/write.rs:957.
Established: ran it; the plaintext was found in the file bytes, the encrypted flag was true, and decrypting gave `InvalidPassword`.
Fix: keep raw-copy AES metadata out of the public options.

### `by_index_seek` and `by_name_seek` return ciphertext without any error
Location: src/read.rs:1087-1103.
Problem: `by_name` demands a password but the seek path does not check `encrypted`.
Established: ran it; the ZipCrypto entry returned 21 bytes and the AES entry 37 bytes of raw data, both Ok.
Fix: return an error for encrypted entries.

### ZipCrypto writer reuses the keystream
Location: src/write.rs:1049.
Problem: the 12-byte header is all zeros rather than random, so every file with the same password has the same keystream, and 12 bytes of plaintext are known. This feature is already labelled insecure, but this is much weaker than normal ZipCrypto.
Established: ran it; two entries with the same content had byte-identical ciphertext.
Fix: use random header bytes.

### `merge_archive` silently replaces an existing entry and leaves the writer marked as writing
Location: src/write.rs:1267 and 1259.
Problem: other paths reject duplicate names, but this one keeps only the merged entry. `is_writing_file()` stays true, and stray `write` calls return Ok(4).
Established: ran it; len was 1 and "first" was lost.
Fix: reject duplicates and reset the state.

### `new_append` leaves the old central directory in place
Location: src/write.rs:637-640.
Problem: new data starts after the old directory, so stream readers stop there and miss new entries, and the file grows with each append.
Established: ran it; the file held 3 central headers and the stream reader saw only `old`. Python saw both entries.
Fix: seek to the directory start before writing, if that is safe.

### `new_append` silently alters other entries' metadata
Location: src/write.rs:2107, src/types.rs:850 and 910-912.
Problem: three changes.
- A CP437 comment is rewritten as UTF-8 without the flag, so "café" becomes "caf├⌐".
- Name bytes C3 A9 gain the UTF-8 flag, so "├⌐" becomes "é".
- An invalid date becomes the current time.
Established: ran it.
Fix: keep the raw fields and flags.

### ZIP64 comment does not round-trip
Location: src/write.rs:1569 and 1598, src/spec.rs:365.
Problem: the comment forces a ZIP64 footer, but the EOCD has no marker values, so the reader never looks for it.
Established: ran it; `zip64_comment()` returned None.
Fix: write marker values, or also try the locator when it is present.

### EOCD entry count handling gives wrong results
Location: src/read.rs:516.
Problem: the 32-bit path uses the on-disk count, while the ZIP64 path uses the total. Nothing checks the count against the directory size.
Established: ran it.
- On-disk count 0 with total 2 gave `Ok(0)`, silently empty; `unzip` and Python see both files.
- Count 1 gave 1 of 2 with no error; `unzip` reports an error.
- Count 3 was rejected; Python opens it.
Fix: use the total and cross-check it against the directory size.

### Truncated EOCD comment rejects the archive
Location: src/spec.rs:333-337.
Established: ran it; "EOCD comment exceeds file boundary", while `unzip` and Python both open the file.
Fix: keep the bytes available instead of failing.

### Windows-style backslash paths are extracted literally on Unix
Location: src/types.rs:594 and 602, src/spec.rs:809.
Problem: `is_dir` and `mangled_name` treat `\` as a separator but the extract path does not.
Established: ran it; `d\` and `d\f.txt` became files or directories with those literal names.
Fix: normalise separators consistently.

### Empty file with Xz compression is written corrupt
Location: src/write.rs:1836-1842 (likely a dependency fault).
Established: ran it; this crate gave "index CRC32 mismatch" and Python `lzma` gave "Corrupt input data". Stored, Deflate, Bzip2, Zstd and PPMd passed the same round-trip test. Sizes 1, 17 and 200,000 also passed with Xz.
Fix: special-case empty input or update the dependency.

### Valid Implode trees with 256 length bytes fail, and debug builds panic
Location: src/legacy/implode.rs:29.
Problem: `(byte + 1)` overflows u8. Release wraps to 0 and returns "not enough codeword lengths", and debug panics.
Established: ran it with first byte 0xFF, and fuzzing hit it too. I used a crafted payload rather than a real file. That a real tree can need all 256 bytes comes from reading the format.
Fix: `byte as usize + 1`.

### Crafted input panics debug builds
Location: src/aes.rs:70 (`compressed_size` underflow, also through `get_aes_verification_key_and_salt`), src/legacy/huffman.rs:141, src/read.rs:996, src/read.rs:1668.
Problem: release builds wrap or skip these checks and mostly return errors. `read.rs:996` returned `Ok(false)` in release despite a huge size, and `read.rs:1668` is a `debug_assert`.
Established: ran the probes in debug and release, and the fuzz run confirmed aes.rs:70 and huffman.rs:141. The input for 1668 was the entry name `a/../b` through `extract_unwrapped_root_dir`.
Fix: use checked arithmetic returning errors, and compute the root directory from simplified components.

### Unsafe code lacks SAFETY comments
Location: src/spec.rs:157, 162 and 167, plus 7 `unsafe impl Pod` blocks.
Problem: the code appears sound, but the justification is missing.
Established: clippy counted 3 blocks and 7 impls. Read the code. `cargo miri test --lib spec::` passes, but that filter runs only one test.
Fix: add the comments.

## Minor

### DOS read-only entries lose file-type bits in `unix_mode`
Location: src/types.rs:632.
Established: ran it; 444 versus 100664 for a writable entry.
Fix: `mode &= !0o222`.

### `by_path` cannot find some entries
Location: src/read.rs:1069.
Established: ran it; `d/` and `./dot` return not found while `by_name` works.
Fix: fall back to a literal match, or try the directory form.

### `decompressed_size` returns None for data-descriptor entries
Location: src/read.rs:566.
Problem: the central directory has the real sizes.
Established: ran it.
Fix: remove that early return.

### `add_directory("")` writes an entry that cannot be extracted
Location: src/write.rs:1445.
Established: ran it; the name is "/" and extraction fails with "Invalid file path".
Fix: reject empty names.

### Failed `add_extra_data` poisons the options
Location: src/write.rs:315-316 and 358.
Problem: an invalid field stays in place, so later valid additions fail. The ID in the error also prints as decimal.
Established: ran the poisoning; read the format string.
Fix: validate before appending.

### Error text and matching are weak
Location: src/result.rs:51 and 25.
Problem: `Display` for `Io` prints only "i/o error", as seen in probe output. `UnsupportedArchive` and `InvalidArchive` require string matching.
Fix: include the cause and add kinds.

### Docs and constants are out of step with behaviour
Location: src/write.rs:430-433 and 445-448, src/compression.rs:247, src/read.rs:1311.
Problem:
- Level 0 is documented but rejected for Deflate and Bzip2 (ran it).
- The default method text is stale (read).
- `SUPPORTED_COMPRESSION_METHODS` omits readable Lzma and legacy methods (ran it).
- A no-op seek per entry discards `BufReader` (20,000 entries gave 20,003 reads and 60,006 seeks).
Fix: correct the docs and list, and remove the seek.

### Public API limits growth
Location: src/extra_fields/mod.rs, src/types.rs:490, src/write.rs:879, src/types.rs:287.
Problem: `ExtraField` is exhaustive, `ZipFileData` exposes public fields, and two `unsafe fn`s have no memory-safety contract.
Established: read the code.
Fix: add `#[non_exhaustive]`, add accessors, and remove `unsafe`.

### Hygiene
Problem:
- 59 missing `# Errors` and 5 missing `# Panics` sections.
- 33 `unwrap` warnings and 3 types without `Debug`.
- 4 glob re-exports (src/extra_fields/mod.rs:23 and 25, src/read.rs:33, src/unstable.rs:10).
- rustdoc `-D warnings` fails on 5 broken links.
- clippy `-D warnings` fails on one collapsible `if`, which looks like toolchain drift.
Established: clippy and rustdoc runs.

## Note

### `xz-static` enables `lzma`, not `xz`
Location: crate/Cargo.toml:87.
Established: read it.

### Full `cargo test` needs a doctest that allocates over 4 GiB
Location: src/spec.rs:114.
Problem: the gate's test step failed here, and the brief says this doctest exceeds the container's memory limit. I did not separately confirm that this is the cause. Skipping it passes everything.

## Verification

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: fail, one lint.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: fail, broken links.
- `cargo test --all-features`: failed in the gate. `--lib --tests` and `--doc -- --skip ZIP64_BYTES_THR` showed no failures.
- `cargo check --no-default-features --features X` for 15 single-feature cases: pass.
- `cargo +nightly-2026-10-01 miri test --no-default-features --lib spec::`: pass, 1 test.
- Mutation fuzz of reader, stream, raw copy, merge and append: about 12 million iterations in a debug build. It used a custom allocator that panics on allocations over 64 GiB. Panics are reported above, and the earlier 3-seed run aborted early because of that panicking allocator in the append path.
- Writer model test: 900,000 iterations in the first run, 600,000 in the rerun.

Findings: 54. Last clock reading: 29 minutes used of 45.
