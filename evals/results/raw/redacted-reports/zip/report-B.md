# Audit report: zip 5.1.1

Verdict: the crate builds and its own tests pass, but it has serious defects outside what those tests cover. Extraction has security gaps. Crafted archives can panic or abort the process. `new_append`, `merge_archive`, `raw_copy_file` and `deep_copy_file` can corrupt archives. `ZipWriter::new_stream` and `ZipStreamReader` are broken in several ways.

All probes ran through `./run` from a throwaway crate at `probe/`, and `crate/` source was not changed. "Debug" below means overflow checks on. Other tools means `unzip` 6 and Python `zipfile` in the container. Nothing in the repository addressed me.

## High

**Extraction applies setuid and setgid bits from the archive**
src/read.rs:942 (mode comes from src/types.rs:622).
`ZipArchive::extract` passes the whole archive mode to `chmod`. An entry with mode 0o106755 is extracted with setuid and setgid set, so extracting as root can plant a setuid binary.
ran it; the on-disk mode was 106755.

**Symlinks pointing outside the destination are created, contrary to the docs**
src/read.rs:908 (docs at 768-770).
the docs say links are "only created and followed if the target is within the destination directory". No target check happens before `make_symlink`, so anything that later follows the link leaves the destination.
ran it; `evil -> /etc/passwd` and `evil2 -> ../../../../etc/hostname` were both created and `extract` returned Ok.

**AES authentication is skipped on empty or short data**
src/aes.rs:171 and src/aes.rs:178.
when `data_remaining` is 0 at the start, or the inner read returns 0 early, `read` returns Ok(0) without checking the HMAC. AE-2 also has no CRC, so tampering goes undetected.
ran both cases with the correct password.
- Patching the central `compressed_size` to header-only returned empty content and Ok, although the declared size was 5.
- Patching it past EOF returned 95 bytes of garbage and Ok.
- A control that flipped a ciphertext bit was correctly rejected.

**Symlink entry with a huge declared size panics or aborts extract**
src/read.rs:894 (the same pattern is at src/read/stream.rs:71, currently unreachable).
`Vec::with_capacity(file.size() as usize)` uses the attacker's Zip64 size.
ran it, in debug and release. Size `u64::MAX` panics with "capacity overflow". Size 2^62 aborts the process with "memory allocation of 4611686018427387904 bytes failed".

**Shrink stream overflows the stack**
src/legacy/shrink.rs:141.
`read_code` recurses once per control code with no depth limit. This is an uncatchable abort with the `legacy-zip` feature.
ran it, in debug and release. A 45 KB payload (20,000 partial-clear pairs) was fine. 200,000 pairs (about 450 KB) gave "has overflowed its stack" and abort.

**Legacy decoders panic on a large declared size**
src/legacy/implode.rs:97 and 214, src/legacy/reduce.rs:107, src/legacy/shrink.rs:227 and 340.
`reserve(uncompressed_size as usize)` trusts the header, so reading a crafted entry panics. Fuzzing also showed the panic in the stream path.
ran it; Zip64 `u64::MAX` gave "capacity overflow" for Shrink, Reduce and Implode, in debug and release.

**Xz entry with a crafted index aborts the process**
src/compression.rs:444 (the allocation is in lzma-rust2 0.13).
untrusted Xz data goes to a dependency that allocates from the stream's record count. Xz is a default feature.
ran it; a 13-byte stream header plus a record count of 2^56 gave "memory allocation of 1152921504606846976 bytes failed" and abort. Fuzzing also hit "capacity overflow".

**`new_append` then `finish` aborts on a long non-ASCII entry comment**
src/types.rs:938.
the comment is decoded from CP437 to UTF-8, which can expand past 65535 bytes, and `try_into().unwrap()` panics. `Drop` then panics again, so the process aborts.
ran it; a 30,000-byte comment of 0xB0 gave `TryFromIntError`, then "panic in a destructor during cleanup", exit 134.

**A file name over 65535 bytes aborts the process**
src/types.rs:902 and 934.
`start_file` panics on `unwrap`, then `Drop` runs `block()` and panics again.
ran it; a 70,000-character name gave panics at 902 and 934, then abort.

**`new_append` and `merge_archive` lose Zip64 data**
src/write.rs:2094; the field is stripped at src/read.rs:1433-1441.
reading removes the Zip64 extra field, and the central directory writer never regenerates it. Any existing entry that needs Zip64 comes back wrong or unreadable. `merge_archive` past 4 GiB has the same problem.
ran it on a real 4.36 GB sparse archive.
- After `new_append`, "big" reported size 4294967295, and "after" (offset 4362076213) failed with "Invalid local file header".
- An entry merged past 4 GiB failed the same way.

**`new_append` makes archives with Info-ZIP Unicode paths unreadable by this crate**
src/read.rs:1538-1542, written at src/write.rs:2098-2101.
the raw name is replaced by the UTF-8 name but the 0x7075 field is kept, so its CRC no longer matches on the next open.
ran it; the name was "café.txt" before, and reopening gave `InvalidArchive("Could not find EOCD")`. Python still opens the file.

**`new_append` clears general-purpose flag bits on existing entries**
src/types.rs:849-865.
`flags()` keeps only three bits, so Implode's dictionary and tree bits are lost.
ran it on tests/data/legacy/implode.zip; flags went from 6 to 0, and `first.txt` then failed with "too many codeword lengths". Before the append it read 1092 bytes.

**`raw_copy_file` of a large entry writes sizes of 0**
src/write.rs:920 and 1088.
the Zip64 field is built with zero sizes, and the raw path skips the later update. The central size fields then point to the zeroed Zip64 values.
ran it; the source size was 5368709120, and after copying `size()` was 0.

**`raw_copy_file` of a ZipCrypto entry clears the encrypted flag**
src/types.rs:716 (options built at src/read.rs:1832).
`options()` carries no ZipCrypto state, so ciphertext is marked as plaintext and can no longer be decrypted.
ran it; the flag was false after the copy. Both a plain read and `by_index_decrypt` failed with "Invalid checksum" after 19 bytes. The AES control worked.

**`large_file(true)` overwrites sizes with 0xFFFFFFFF, breaking later use**
src/write.rs:2078-2079; the deep-copy clamp is at src/write.rs:684-687.
in-memory metadata is replaced by the marker value, with two effects.
- `finish_into_readable` reads past the entry.
- `deep_copy_file` copies following entries too, and rewrites the source's size.
ran both.
- A 5-byte entry reported size 4294967295 and failed with "Invalid checksum".
- With entries a, b and a deep copy of a to c, both a and c failed. Reopening with `ZipArchive::new` was fine.
- A 600,000-iteration writer model test failed on this deep-copy path. A 200,000-iteration rerun without `large_file` passed.

## Medium

**`ZipStreamReader` never reaches the metadata stage**
src/read.rs:1944-1948 and src/read/stream.rs:48.
the 30 bytes read to detect the central directory are consumed, so the next parse sees the wrong magic and the `while let Ok` ends silently. Metadata is never visited. As a result, `extract` loses permissions and writes symlinks as regular files.
ran it; 2 files gave 0 metadata visits, a 750 file became 644, and a symlink became a regular file.

**A test passes for the wrong reason, hiding stream bugs**
src/read/stream.rs:383.
`writer.finish()` returns a cursor at the end, so `extract` fails with EOF rather than on the symlink check. The other stream tests' metadata callbacks never run either.
ran the same pattern; position equalled length (102) and the error was `UnexpectedEof`.

**Stream reader rejects a valid empty archive**
src/read.rs:1944.
it always reads 30 bytes, but an empty zip is 22 bytes.
ran it; `ZipArchive` reports length 0, the stream reader returns `UnexpectedEof`.

**Stream reader cannot read data-descriptor entries, including this crate's own output**
src/types.rs:782-786.
common streamed zips are rejected, and neither `read_zipfile_from_stream` nor `ZipStreamReader` documents it.
ran it on `new_stream` output; the error was "The file length is not available in the local header".

**`ZipFile::data_start()` panics on stream entries**
src/read.rs:1818 (docs at 1935 say "set to 0").
ran it; "called `Option::unwrap()` on a `None` value".

**Unicode extra fields can reject a whole archive**
src/read.rs:1530 and 1538-1541, src/extra_fields/zipinfo_utf8.rs:20.
there are three cases.
- A stale CRC is an error, but APPNOTE says to ignore the field.
- Invalid UTF-8 content is an error.
- The comment CRC is computed over the decoded comment rather than the raw bytes, so a valid field on a CP437 comment fails.
ran all three; each gave "Could not find EOCD". An ASCII-comment control passed. `unzip` and Python accepted the stale-CRC and CP437-comment files, the two I tried with them.

**NTFS and extended timestamp variants reject a whole archive**
src/extra_fields/ntfs.rs:33-52, src/extra_fields/extended_timestamp.rs:43.
these fields are optional metadata, yet NTFS with an extra attribute (length 36) or reserved bytes only (length 4), and UT with length 9 and flags 1, fail the whole file.
ran it; all were rejected. `unzip` and Python opened the length-36 and UT files.

**Real errors are replaced by "Could not find EOCD"**
src/read.rs:660-666.
every error from central directory parsing is discarded, including I/O failures and multi-disk. Callers cannot tell a disk error from a bad file.
ran it; a reader returning `PermissionDenied` during the central directory, and a multi-disk EOCD, both gave `InvalidArchive("Could not find EOCD")`.

**EOCD retries give quadratic work**
src/read.rs:650-666, src/spec.rs:619.
each fake EOCD triggers a re-parse of the central directory.
ran it; 69 KB took 0.12 s, 138 KB 0.42 s, 276 KB 1.17 s and 552 KB 4.83 s, about 4x per doubling.

**`has_overlapping_files` is quadratic**
src/read.rs:997.
ran it on legitimate archives; 40k entries took 0.23 s, 80k 0.89 s and 160k 3.53 s.

**Following any symlink always fails with "Invalid base path"**
src/read.rs:1718-1723.
`simplified_components` returns None for an absolute base path, so the check always errors and the `is_absolute_enclosed` code is dead. The docs promise that safe links are followed and that files are overwritten.
ran three cases.
- Extracting the same archive twice with a symlink failed on the second run.
- A file through an inner relative link failed with "Invalid base path".
- An absolute link inside the destination failed with "Invalid symlink target path".

**Unix-made entries with zero high attribute bits extract as mode 000**
src/types.rs:622.
`external_attributes` 0x20 with system Unix gives `unix_mode() == Some(0)`, and the file becomes inaccessible. `unzip` has code that treats a zero mode as absent, from my memory of its source; I did not run this case.
ran it; the on-disk mode was 0.

**Directory modes are ignored, and the extraction privacy claim is false**
src/read.rs:898-900 and 315-329, docs at 773-775.
directories never get their archive mode. The docs also say partly extracted new directories are not usable by other users, but they use the umask default.
ran it; 0700 and 0555 directories both became 755. After a failed extract, the new directories were 755.

**Correct ZipCrypto password rejected when the date is invalid**
src/read.rs:414.
the check byte needs the raw time, but `last_modified_time` is None for an invalid date, so 0 is used.
ran it; a hand-encrypted entry decrypted with a valid date and gave `InvalidPassword` with date field 0.

**`new_stream` with ZipCrypto produces entries nobody can open**
src/zipcrypto.rs:168 and src/write.rs:1009.
the check byte is taken from the CRC while bit 3 is set, and readers then expect the time byte.
ran it, across six compression methods. This crate gave "provided password is incorrect", Python gave "Bad password" and `unzip` skipped the file. The non-streaming output decrypted fine.

**`new_stream` with AES always fails at finish**
src/write.rs:1117 and 2019.
the AES field update seeks backward. `start_file` and writes succeed, then all data is lost at the end. The error text is only "i/o error".
ran it across all compression methods tested.

**`new_stream` with `large_file` over 4 GiB fails at finish**
src/write.rs:2053.
this branch only runs for large files, yet it errors with "Large file option has not been set". The check looks inverted.
ran it; 4311744512 bytes to a sink, and finish returned an error.

**`new_stream` continues after a failed `abort_file` and corrupts an earlier entry**
src/write.rs:1175-1179.
the entry is popped, then the seek fails before `writing_to_file` is reset, so the next `finish_file` overwrites the previous file's sizes and CRC.
ran it; a, b, abort, c gave "a" failing with "Invalid checksum" after 12 bytes.

**`new_stream` sets bit 3 without writing a descriptor**
src/write.rs:1009, with the missing writes at 1451 (directories) and 1088 (raw copy).
ran it; `unzip -t` reports "invalid zip file with overlapped components (possible zip bomb)" for both the directory and raw-copy cases. Plain files were fine, and Python accepted the archives.

**Raw copies lose file type and metadata**
src/read.rs:1836, src/types.rs:738 and 849-865.
the docs say metadata is copied, but several things change.
- Symlinks become regular files (mode 100777, `is_symlink` false).
- Directories become 100755.
- Comments and extra fields are dropped.
- Implode flags go from 6 to 0.
ran it.

**`start_file` with options from a parsed AES entry stores plaintext marked as encrypted**
src/read.rs:1845-1849 and src/write.rs:957.
ran it; the plaintext was found in the file bytes, the encrypted flag was true, and decrypting gave `InvalidPassword`.

**`by_index_seek` and `by_name_seek` return ciphertext without any error**
src/read.rs:1087-1103.
`by_name` demands a password but the seek path does not check `encrypted`.
ran it; the ZipCrypto entry returned 21 bytes and the AES entry 37 bytes of raw data, both Ok.

**ZipCrypto writer reuses the keystream**
src/write.rs:1049.
the 12-byte header is all zeros rather than random, so every file with the same password has the same keystream, and 12 bytes of plaintext are known. This feature is already labelled insecure, but this is much weaker than normal ZipCrypto.
ran it; two entries with the same content had byte-identical ciphertext.

**`merge_archive` silently replaces an existing entry and leaves the writer marked as writing**
src/write.rs:1267 and 1259.
other paths reject duplicate names, but this one keeps only the merged entry. `is_writing_file()` stays true, and stray `write` calls return Ok(4).
ran it; len was 1 and "first" was lost.

**`new_append` leaves the old central directory in place**
src/write.rs:637-640.
new data starts after the old directory, so stream readers stop there and miss new entries, and the file grows with each append.
ran it; the file held 3 central headers and the stream reader saw only `old`. Python saw both entries.

**`new_append` silently alters other entries' metadata**
src/write.rs:2107, src/types.rs:850 and 910-912.
three changes.
- A CP437 comment is rewritten as UTF-8 without the flag, so "café" becomes "caf├⌐".
- Name bytes C3 A9 gain the UTF-8 flag, so "├⌐" becomes "é".
- An invalid date becomes the current time.
ran it.

**ZIP64 comment does not round-trip**
src/write.rs:1569 and 1598, src/spec.rs:365.
the comment forces a ZIP64 footer, but the EOCD has no marker values, so the reader never looks for it.
ran it; `zip64_comment()` returned None.

**EOCD entry count handling gives wrong results**
src/read.rs:516.
the 32-bit path uses the on-disk count, while the ZIP64 path uses the total. Nothing checks the count against the directory size.
ran it.
- On-disk count 0 with total 2 gave `Ok(0)`, silently empty; `unzip` and Python see both files.
- Count 1 gave 1 of 2 with no error; `unzip` reports an error.
- Count 3 was rejected; Python opens it.

**Truncated EOCD comment rejects the archive**
src/spec.rs:333-337.
ran it; "EOCD comment exceeds file boundary", while `unzip` and Python both open the file.

**Windows-style backslash paths are extracted literally on Unix**
src/types.rs:594 and 602, src/spec.rs:809.
`is_dir` and `mangled_name` treat `\` as a separator but the extract path does not.
ran it; `d\` and `d\f.txt` became files or directories with those literal names.

**Empty file with Xz compression is written corrupt**
src/write.rs:1836-1842 (likely a dependency fault).
ran it; this crate gave "index CRC32 mismatch" and Python `lzma` gave "Corrupt input data". Stored, Deflate, Bzip2, Zstd and PPMd passed the same round-trip test. Sizes 1, 17 and 200,000 also passed with Xz.

**Valid Implode trees with 256 length bytes fail, and debug builds panic**
src/legacy/implode.rs:29.
`(byte + 1)` overflows u8. Release wraps to 0 and returns "not enough codeword lengths", and debug panics.
ran it with first byte 0xFF, and fuzzing hit it too. I used a crafted payload rather than a real file. That a real tree can need all 256 bytes comes from reading the format.

**Crafted input panics debug builds**
src/aes.rs:70 (`compressed_size` underflow, also through `get_aes_verification_key_and_salt`), src/legacy/huffman.rs:141, src/read.rs:996, src/read.rs:1668.
release builds wrap or skip these checks and mostly return errors. `read.rs:996` returned `Ok(false)` in release despite a huge size, and `read.rs:1668` is a `debug_assert`.
ran the probes in debug and release, and the fuzz run confirmed aes.rs:70 and huffman.rs:141. The input for 1668 was the entry name `a/../b` through `extract_unwrapped_root_dir`.

**Unsafe code lacks SAFETY comments**
src/spec.rs:157, 162 and 167, plus 7 `unsafe impl Pod` blocks.
the code appears sound, but the justification is missing.
clippy counted 3 blocks and 7 impls. Read the code. `cargo miri test --lib spec::` passes, but that filter runs only one test.

## Low

**DOS read-only entries lose file-type bits in `unix_mode`**
src/types.rs:632.
ran it; 444 versus 100664 for a writable entry.

**`by_path` cannot find some entries**
src/read.rs:1069.
ran it; `d/` and `./dot` return not found while `by_name` works.

**`decompressed_size` returns None for data-descriptor entries**
src/read.rs:566.
the central directory has the real sizes.
ran it.

**`add_directory("")` writes an entry that cannot be extracted**
src/write.rs:1445.
ran it; the name is "/" and extraction fails with "Invalid file path".

**Failed `add_extra_data` poisons the options**
src/write.rs:315-316 and 358.
an invalid field stays in place, so later valid additions fail. The ID in the error also prints as decimal.
ran the poisoning; read the format string.

**Error text and matching are weak**
src/result.rs:51 and 25.
`Display` for `Io` prints only "i/o error", as seen in probe output. `UnsupportedArchive` and `InvalidArchive` require string matching.

**Docs and constants are out of step with behaviour**
src/write.rs:430-433 and 445-448, src/compression.rs:247, src/read.rs:1311.

- Level 0 is documented but rejected for Deflate and Bzip2 (ran it).
- The default method text is stale (read).
- `SUPPORTED_COMPRESSION_METHODS` omits readable Lzma and legacy methods (ran it).
- A no-op seek per entry discards `BufReader` (20,000 entries gave 20,003 reads and 60,006 seeks).

**Public API limits growth**
src/extra_fields/mod.rs, src/types.rs:490, src/write.rs:879, src/types.rs:287.
`ExtraField` is exhaustive, `ZipFileData` exposes public fields, and two `unsafe fn`s have no memory-safety contract.
read the code.

**Hygiene**

- 59 missing `# Errors` and 5 missing `# Panics` sections.
- 33 `unwrap` warnings and 3 types without `Debug`.
- 4 glob re-exports (src/extra_fields/mod.rs:23 and 25, src/read.rs:33, src/unstable.rs:10).
- rustdoc `-D warnings` fails on 5 broken links.
- clippy `-D warnings` fails on one collapsible `if`, which looks like toolchain drift.
clippy and rustdoc runs.

## Low

**`xz-static` enables `lzma`, not `xz`**
crate/Cargo.toml:87.
read it.

**Full `cargo test` needs a doctest that allocates over 4 GiB**
src/spec.rs:114.
