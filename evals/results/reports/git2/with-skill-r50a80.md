# Audit report: git2 0.20.2 (`crate/src/`)

## Verdict

The crate's own tests pass, but safe code can reach undefined behaviour in many places. Most of it comes from three causes:
- Borrowed data is not tied to the object that owns it.
- Null pointers from libgit2 go straight into slices and `CStr`.
- Options structs store pointers to themselves.

Non-UTF-8 repository data, including data from a remote, panics. Panics in callbacks are often swallowed. Several option setters do something other than what their docs say.

Findings: Blocker 21, Major 20, Minor 13, 54 in total. `crate/` was not changed. No text in the repository addressed me.

Throwaway probes are in `probe/` in the workspace. "ASan" means the probe was built with nightly `-Zsanitizer=address` and libgit2 was compiled with `-fsanitize=address`. "Debug" means the standard library's unsafe-precondition checks were active.

## Blocker

### Credential helper input allows newline injection
Location: src/cred.rs:434-436 (`execute_cmd`), reached through `Cred::credential_helper`.
Problem: the username is written to the helper's stdin with `writeln!` and no newline check. A URL such as `http://a%0Ahost=evil.example@host/` gives a username containing a newline. The helper then sees a second `host=` line, and in the credential protocol the later value wins. This is the same class as CVE-2020-5260, where the helper hands over credentials for another host.
Established by:
- `CredentialHelper::new("https://real.example/repo").username(Some("a\nhost=evil.example"))`: the helper's stdin was `protocol=https\nhost=real.example\nusername=a\nhost=evil.example\n`.
- A local HTTP 401 server showed libgit2 passing `username_from_url = "a\nhost=evil.example"` to the credentials callback.
Fix: refuse any field containing `\n` or NUL, and return no credentials.

### `BlameHunk` signatures dereference null
Location: src/blame.rs:110-112, 131-133.
Problem: hunks for lines added by `Blame::blame_buffer` have a null `final_signature` and `orig_signature`, but a non-optional `Signature` is returned. Calling `.name()` crashes.
Established by: a probe that blamed a buffer with one new line. Debug: "null pointer dereference" abort. Release: SIGSEGV.
Fix: return `Option<Signature>`.

### Crafted index causes a heap over-read
Location: src/index.rs:818-823 (`IndexEntry::from_raw`).
Problem: the path length is taken from the flags bits, not from the NUL-terminated string. An index file whose flags claim 1000 bytes for a 1-byte name makes `Index::get` copy adjacent heap memory into `entry.path`.
Established by: a hand-built index with a valid SHA-1. `path.len()` was 1000 and contained heap bytes. ASan: heap-buffer-overflow, READ of size 1000.
Fix: always use `CStr::from_ptr(path).to_bytes()`.

### Index callbacks crash on an empty pathspec
Location: src/index.rs:736 (`index_matched_path_cb`).
Problem: with an empty pathspec list, libgit2 passes a null `matched_pathspec`, and `CStr::from_ptr` dereferences it. This affects `add_all`, `update_all` and `remove_all`.
Established by: all three with `Vec::<&str>::new()` and a callback gave SIGSEGV in debug. `add_all` in release also gave SIGSEGV.
Fix: treat null as an empty slice.

### Options structs keep a stale pointer to themselves
Location: src/apply.rs:127, 140; src/stash.rs:132.
Problem: `ApplyOptions::hunk_callback`, `ApplyOptions::delta_callback` and `StashApplyOptions::progress_cb` store `self as *mut _` as the payload. Moving the options afterwards, for example returning them from a function, leaves a dangling payload.
Established by: a helper function returning the options, followed by `stash_apply` or `apply`. Results were SIGBUS and SIGSEGV. ASan: stack-use-after-return on both.
Fix: set the payload in `raw()`, as `CheckoutBuilder::configure` does.

### `RevwalkWithHideCb::into_inner` keeps the dangling callback
Location: src/revwalk.rs:43-46.
Problem: `git_revwalk_reset` does not clear the hide callback, and the returned `Revwalk<'repo>` is not bound to `'cb`. Walking after the closure is gone calls it through a dead pointer.
Established by: the closure saw garbage captures (`[2, 197, 196, 107]` instead of `[1, 1, 1, 1]`). ASan: stack-use-after-scope.
Fix: call `git_revwalk_add_hide_cb(raw, None, null)` in `into_inner`.

### `Remote::get_refspec` outlives the remote
Location: src/remote.rs:283.
Problem: it returns `Refspec<'repo>`, but the refspec is owned by the `Remote`.
Established by: reading `.str()` after dropping the remote gave SIGSEGV. ASan: heap-use-after-free.
Fix: return `Option<Refspec<'_>>`.

### `RebaseOperation` outlives the `Rebase`
Location: src/rebase.rs:217-224.
Problem: `Iterator::Item` is `RebaseOperation<'rebase>`, where `'rebase` is the repository lifetime. The operation array is freed with the `Rebase`.
Established by: reading the operation after dropping the rebase returned a bogus `kind() == None`. ASan: heap-use-after-free.
Fix: expose this as a lending `next(&mut self)`, or return owned data.

### `set_workdir` and `set_namespace` free strings still borrowed
Location: src/repo.rs:504 with 520; 537-546 with 549-568.
Problem: `workdir()` and `namespace()` return borrows from `&self`, and the setters also take `&self` and free the old string.
Established by:
- `namespace` silently changed to the new value.
- `workdir` printed garbage bytes.
- ASan: heap-use-after-free on both.
Fix: make the setters take `&mut self`.

### `get_attr` returns data freed by the next lookup
Location: src/repo.rs:1089-1128.
Problem: the returned `&str` or `&[u8]` lives in the attribute cache. Another `&self` call after `.gitattributes` changes reloads the file and frees that memory.
Established by: ASan heap-use-after-free, READ of size 16, reading the first value after the second `get_attr`.
Fix: return owned bytes plus the attribute kind.

### `Indexer` does not borrow its `Odb`
Location: src/indexer.rs:126.
Problem: `Indexer<'a>` ties `'a` to the `Odb`'s parameter, not to the `&Odb` borrow, and libgit2 stores the pointer without a reference count.
Established by: an indexer built inside a block with `Odb::new()`, then written with verify on, gave SIGSEGV. ASan: heap-use-after-free.
Fix: take `Option<&'a Odb<'_>>`.

### `Pathspec::match_diff` list outlives the diff
Location: src/pathspec.rs:63-79 (read through `diff_entry`).
Problem: the list stores pointers to diff deltas, but its lifetime is tied only to the `Pathspec`.
Established by: ASan heap-use-after-free on `list.diff_entry(0)` after the diff was dropped.
Fix: add a second lifetime for the diff.

### `Reflog` outlives its repository
Location: src/reflog.rs:11; src/repo.rs:2638.
Problem: `Reflog` has no lifetime, but its refdb backend keeps a raw repository pointer.
Established by: `log.write()` after the repository was dropped. ASan: heap-use-after-free.
Fix: add a `'repo` lifetime.

### Calling `OdbWriter::finalize` twice crashes
Location: src/odb.rs:390-398.
Problem: it takes `&mut self`, so a second call is safe code, and it reuses the finished stream.
Established by: the second `finalize` on a loose-backed stream gave SIGSEGV, also under ASan at a null pc.
Fix: consume `self`, or record that finalize has run and return `Err`.

### `Buf` deref on an empty buffer is undefined behaviour
Location: src/buf.rs:47, 53.
Problem: `Buf::new()` has a null `ptr`, and `Deref` passes it to `slice::from_raw_parts`.
Established by: `Buf::new().len()` aborts in debug with an unsafe-precondition violation. Release returned 0, which is silent undefined behaviour.
Fix: return `&[]` when the pointer is null.

### `Remote::list` on an empty remote is undefined behaviour
Location: src/remote.rs:387.
Problem: a null base pointer with size 0 goes into `from_raw_parts`.
Established by: connecting to an empty bare repository and calling `list()` aborts in debug. Release returned `Ok(0)`.
Fix: check for a null pointer.

### `DiffBinaryFile::data` is undefined behaviour without data
Location: src/diff.rs:1281.
Problem: `data` is null for ordinary binary diffs that lack `show_binary`, and also for a parsed patch with an empty `literal 5` body.
Established by: `Diff::foreach` with a binary callback aborts in debug, and the harness hit the same abort from a patch input. Release returned 0.
Fix: check for a null pointer.

### `stash_foreach` crashes on a reflog entry without a message
Location: src/stash.rs:163.
Problem: `CStr::from_ptr(message)` is called on null.
Established by: removing the message from `logs/refs/stash` gave SIGSEGV.
Fix: check for null and pass an empty string or an `Option`.

### Non-UTF-8 ref names from a remote panic the client
Location: src/remote.rs:490, 495; src/remote_callbacks.rs:401, 451, 455, 426.
Problem: `RemoteHead::name`, `symref_target` and the `update_tips`, `push_update_reference` and certificate callbacks all call `from_utf8(..).unwrap()` on remote-controlled bytes.
Established by:
- A local remote with branch `caf\xe9` panicked `head.name()` at remote.rs:490.
- The same remote panicked `fetch` with `update_tips` at remote_callbacks.rs:401.
- Lines 451, 455, 426 and remote.rs:495 were only read.
Fix: add byte accessors, and have `name()` return `Option<&str>` or an error.

### `DiffFile::mode` panics on modes from patches and trees
Location: src/diff.rs:629.
Problem: unknown modes reach `panic!`, undocumented. `Diff::from_buffer` accepts any octal mode, and a tree object can carry unusual ones.
Established by:
- `Diff::from_buffer(b"diff --git a/f b/f\nold mode 100600\nnew mode 100777\n")` panicked with `unknown mode: 33152`.
- The harness also reached this panic through a tree diff.
Fix: add an `Unknown` variant, or return `Option<FileMode>`.

### Non-UTF-8 local repository data panics
Location: src/reference.rs:498; src/describe.rs:38; src/worktree.rs:115; src/repo.rs:576, 83, 2365; src/stash.rs:163; src/message.rs:32.
Problem: these sites call `unwrap` on `from_utf8` for repository data or caller bytes.
Established by, each panicking at the line shown:
- `references().names()` with a `caf\xe9` ref.
- `Describe::format` with a `caf\xe9` tag.
- `Worktree::is_locked` with a Latin-1 lock reason.
- `Repository::message` with a Latin-1 `MERGE_MSG`.
- `stash_foreach` and `fetchhead_foreach` with Latin-1 text.
- `message_prettify(&b"caf\xe9\n"[..], None)`.
repo.rs:2365 (`note_default_ref`) was only read.
Fix: return `Err` or `None`, and add byte variants.

## Major

### Callback panics are swallowed and resurface later
Location: src/odb.rs:540, 456-463, 490-499; src/repo.rs:64, 97; src/stash.rs:174; src/build.rs:699; src/indexer.rs:192-197.
Problem: `try_call!` calls `panic::check` only when the return value is negative, and these paths return a positive value or skip `try_call!` altogether. The call returns `Ok(())`. Later callbacks on that thread are skipped because `panic::wrap` refuses to run them.
Established by:
- `Odb::foreach(|_| panic!())` returned `Ok(())`.
- A second foreach visited 0 objects.
- The process then exited silently at an unrelated failing `Repository::open`.
- `stash_foreach`, a checkout notify panic and `OdbPackwriter::write` behaved the same way.
Fix: call `panic::check()` after every callback-taking call.

### `tag_foreach` ignores the libgit2 result
Location: src/repo.rs:2057-2063.
Problem: the return value is discarded and `Ok(())` is always returned, so errors are lost.
Established by: a panicking callback gave `Ok(())`, and a second call visited 0 tags.
Fix: use `try_call!`.

### `fetchhead_foreach` drops valid `FETCH_HEAD` files
Location: src/repo.rs:79-80.
Problem: `assert!(!ref_name.is_null())` fires for lines libgit2 parses with no ref name. The panic is then swallowed.
Established by: `<oid>\n`, `<oid>\t\t<url>` and `remote-tracking branch 'o/m' of url` each returned `Ok(())` with 0 entries. From memory, not checked here, `<oid>\t\t<url>` is the form `git fetch <url>` writes.
Fix: pass `Option` values to the callback.

### `Tree::walk` aborts on a non-UTF-8 directory
Location: src/tree.rs:219-222.
Problem: a root that is not UTF-8 returns -1 and aborts the whole walk, with no error set.
Established by: a tree with `caf\xe9/f` and `g` stopped before `g`, and the error was a stale, unrelated `reference 'refs/heads/master' not found`.
Fix: skip the node, or pass bytes to the callback.

### Cancelling a checkout from `notify` still returns `Ok`
Location: src/build.rs:693-699; src/repo.rs:2128.
Problem: `false` maps to 1, which `try_call!` treats as success, although the docs say the checkout is cancelled.
Established by: `checkout_tree` with `notify` returning false gave `Ok(())` and the file was left unchanged.
Fix: return a negative code such as `GIT_EUSER`.

### `MergeOptions::skip_reuc` sets the wrong flag
Location: src/merge.rs:98.
Problem: it sets `GIT_MERGE_FAIL_ON_CONFLICT` instead of `GIT_MERGE_SKIP_REUC`.
Established by: a conflicted `merge_commits` returned `Ok` with conflicts by default, and `Err(MergeConflict)` with `skip_reuc(true)`.
Fix: use the correct constant.

### `CheckoutBuilder::dry_run` does not check anything
Location: src/build.rs:350-354.
Problem: it sets `GIT_CHECKOUT_NONE`, not `GIT_CHECKOUT_DRY_RUN`. The docs promise conflict checking. The flag also survives later calls, because the mask does not clear bit 30.
Established by:
- A safe checkout gave `Err(Conflict)`.
- `dry_run()` on the same state gave `Ok(())`.
- `dry_run().force()` gave `Ok(())` and changed nothing.
Fix: use `GIT_CHECKOUT_DRY_RUN` and clear it in `force()` and `safe()`.

### `force()` and `safe()` clear `recreate_missing`
Location: src/build.rs:351, 359, 369.
Problem: the mask `(1 << 4) - 1` includes `GIT_CHECKOUT_RECREATE_MISSING` (bit 2).
Established by: `recreate_missing(true).safe()` left the file missing, and `safe().recreate_missing(true)` restored it.
Fix: mask only the `FORCE` bit.

### `PackBuilder::hash` has inverted logic
Location: src/packbuilder.rs:190.
Problem: it returns a hash only when `object_count() == 0`.
Established by: a written pack with 3 objects gave `hash=None`.
Fix: invert the check, or use the pack name (the method is deprecated).

### `RebaseOperation::exec` panics on every non-exec operation
Location: src/rebase.rs:319.
Problem: `opt_bytes(..).unwrap()` is called on a null pointer, although the method returns `Option`.
Established by: `op.exec()` on a `Pick` panicked.
Fix: use `and_then`.

### `AnnotatedCommit::refname` panics when there is no ref
Location: src/merge.rs:56.
Problem: `find_annotated_commit` leaves the ref name null. The docs say `None` means only invalid UTF-8.
Established by: `refname()` panicked.
Fix: make `refname_bytes` return `Option`.

### `Diff::format_email` panics on multi-line summaries
Location: src/diff.rs:273, also 267, 268, 271.
Problem: the summary joins the first paragraph's lines, so `message.starts_with(summary)` fails.
Established by: the message `"line one\nline two\n\nbody\n"` panicked at line 273. Lines 267, 268 and 271 were only read.
Fix: return `Err` and find the body another way.

### `CredentialHelper` drops IP-address hosts
Location: src/cred.rs:211.
Problem: only `Host::Domain` is kept, so `host=` is omitted and URL-scoped config is skipped.
Established by: `https://10.0.0.1:8443/repo` gave helper stdin of `protocol=https\n`.
Fix: use `host_str()`.

### `CredentialHelper` reads only one `credential.helper`
Location: src/cred.rs:256-265.
Problem: the comment says "all", and from memory gitcredentials(7) says each configured helper is tried in turn, but `get_string` returns only the last value.
Established by: a working helper followed by a failing one gave `None`.
Fix: use `Config::multivar` and honour an empty value as a reset.

### `Index::get_path` and `Pathspec::matches_path` panic on ordinary paths
Location: src/index.rs:397; src/pathspec.rs:162.
Problem: they call `path_to_repo_path(path).unwrap()`, although the return types are `Option` and `bool`.
Established by: `get_path("")`, `get_path("/etc/passwd")` and `matches_path("")` panicked.
Fix: return `None` and `false`.

### Interior NUL bytes panic instead of returning an error
Location:
- Validators and matchers: reference.rs:70; remote.rs:98; tag.rs:24; tree.rs:173; refspec.rs:42, 60; lib.rs:936.
- Functions returning `Result`: transaction.rs:41, 62-63, 90-92, 116, 132; refspec.rs:83, 97.
- Builders: build.rs:178, 532, 541, 547, 553, 559, 721, 741; diff.rs:906, 914, 920; status.rs (`pathspec`); stash.rs:49; merge.rs:237, 251, 265; remote.rs:591, 691, 701; repo.rs:3484-3519; rebase.rs:64; describe.rs:99, 162; email.rs:97; proxy_options.rs:35.
Problem: none of these panics is documented. A `#[should_panic]` test on `Reference::is_valid_name` records the behaviour.
Established by: `is_valid_name` on all three types, `Tree::get_name` and `Transaction::lock_ref` panicked. The others were only read.
Fix: validators return `false`, `Result` functions use `?`, and builders store the error until `build`.

### Errors from callbacks lose their message
Location: src/transport.rs:226, 294; src/build.rs:312.
Problem: only the code is returned; `raw_set_git_error` is not called.
Established by:
- A transport factory's "factory says no" became `Err("no error")`.
- `remote_create`'s "create says no" became an unrelated `.git/shallow` stat error.
- transport.rs:294 was only read.
Fix: call `raw_set_git_error`.

### A NUL in error text aborts the process
Location: src/transport.rs:359; src/remote_callbacks.rs:432; src/error.rs:363.
Problem: `CString::new(..).unwrap()` runs outside `panic::wrap`, inside `extern "C"`.
Established by: a custom transport stream returning `io::Error` with `"bad\0byte"` aborted with "panic in a function that cannot unwind". The certificate path was only read.
Fix: replace or strip NUL bytes.

### Safe global setters race with other threads
Location: src/opts.rs:79, 119, 136 (`enable_caching`, `strict_object_creation`, `strict_hash_verification`).
Problem: they write plain `bool` globals without synchronisation, such as `git_cache__enabled`, read elsewhere with no lock. `set_cache_object_limit` is `unsafe` for this same reason.
Established by: reading the code only. A ThreadSanitizer run did not flag these writes.
Fix: mark them `unsafe`.

### `execute_cmd` panics on a blank helper when `sh` is missing
Location: src/cred.rs:392.
Problem: `parts.next().unwrap()` runs on an empty command.
Established by: `credential.helper = !` with `PATH=/nonexistent` panicked.
Fix: return `(None, None)`.

## Minor

### `PackBuilder::write` leaks on every call
Location: src/packbuilder.rs:95-96; error paths at src/odb.rs:165-175 and src/indexer.rs:134-150.
Problem: the `Box::into_raw` payload is never freed.
Established by: a counting allocator showed live allocations grew by exactly 2000 after 2000 calls. The error paths were only read.
Fix: free the box after the call.

### Narrowing casts return the wrong element
Location: src/commit.rs:300, 314; src/blame.rs:65.
Problem: `usize` is cast to `u32` with truncation.
Established by: `commit.parent(1 << 32)` and `parent_id(1 << 32)` returned parent 0. blame.rs:65 was only read.
Fix: use `try_from` and return an error or `None`.

### `Parents` and `ParentIds` do not abort as documented
Location: src/commit.rs:362-363 and 386-389 (the `and_then` iterator bodies).
Problem: the docs say iteration aborts on a missing commit, and the type claims `FusedIterator`. It yields `None` and then `Some`.
Established by: with a missing first parent, `len` was 2, first `None`, second `Some`.
Fix: stop the range on first failure.

### `IndexEntries` panics when another handle shrinks the index
Location: src/index.rs:761.
Problem: `repo.index()` returns handles to one shared index.
Established by: `a.iter()`, then `b.clear()`, then `next()` panicked.
Fix: use `and_then`.

### `Time` equality and ordering disagree, and `Commit::time` loses the sign
Location: src/time.rs:9, 57; src/commit.rs:178.
Problem: `Eq` includes `sign` and `Ord` does not. `Commit::time` also rebuilds the time without `-0000`.
Established by: `time.sign` was `+`, `when.sign` was `-`, `eq` false, `cmp` Equal.
Fix: make `Eq` and `Ord` consistent, and keep the sign.

### Writes go through pointers derived from shared references
Location: src/odb.rs:361; src/buf.rs:63; src/message.rs:141; src/call.rs:78.
Problem: `buf.as_ptr() as *mut` and `&self.raw as *const _ as *mut _` are then written by C. This breaks the aliasing rules, though I saw no misbehaviour.
Established by: reading the code only. Miri cannot run these paths.
Fix: use `as_mut_ptr` and `UnsafeCell`.

### `Send` for `Diff` and `Patch` has no justification
Location: src/diff.rs:25; src/patch.rs:18.
Problem: both hold repository pointers while `Repository` is `!Sync`, so another thread can use the repository concurrently.
Established by: reading the code only. A 4-thread stress under ASan found no crash.
Fix: justify it in a comment or remove the impls.

### Casts rely on `repr(Rust)` layout
Location: src/remote.rs:388; src/remote_callbacks.rs:519; src/object.rs:151, 161; src/cert.rs:97.
Problem: slices and references are reinterpreted between types without `repr(transparent)`.
Established by: reading the code only.
Fix: add `repr(transparent)`.

### Panic can abort inside `extern "C"` in `submodules`
Location: src/repo.rs:966.
Problem: `assert_eq!(rc, 0)` runs with no `panic::wrap`.
Established by: reading the code only. Ten `.gitmodules` shapes did not trigger it.
Fix: return `rc`.

### Undocumented unsafe
Location: crate-wide.
Problem: 798 unsafe blocks and 8 `unsafe impl`s have no SAFETY comment, and 17 `unsafe fn`s have no `# Safety` section.
Established by: advisory clippy run, plus a grep finding SAFETY only in tracing.rs.
Fix: add the comments and sections.

### Missing `# Errors` and `# Panics` sections
Location: crate-wide.
Problem: 428 functions lack `# Errors` and 117 lack `# Panics`.
Established by: advisory clippy run.
Fix: add the sections.

### Missing `Debug` implementations
Location: crate-wide.
Problem: 113 public types do not implement `Debug`.
Established by: advisory clippy run.
Fix: add them, printing no secrets.

### Lint configuration and toolchain drift
Location: src/lib.rs:69 and crate-wide.
Problem:
- `cargo clippy -D warnings` fails on Rust 1.99.
- From the CI config, its rustfmt job runs `cargo fmt`, and my grep of the workflows found no clippy step.
- 16 `#[allow]`s have no reason.
Established by: running clippy and reading `.github/workflows/main.yml`.
Fix: add a lint table and use `#[expect]`.

## Verification

- `cargo fmt --all --check`: pass.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --no-default-features`: pass, run through `check.sh`.
- `cargo clippy --workspace --all-targets --no-default-features -- -D warnings`: fail, also on `-p git2`; toolchain drift, see Minor.
- `cargo test --locked -p git2 --no-default-features`: pass on the default temp dir. Results were 187 unit tests plus doc and integration tests, with 1 ignored.
- `check.sh --audit --features "" /work/crate`: reported 3 failures in `cred::test::credential_helper{4,5,7}`, because I had set `TMPDIR=/tmp` and `/tmp` is mounted `noexec`. I judged this environmental: the cred tests pass (10 of 10) with `TMPDIR` on `/work/tmp`. It is not a finding.
- `python3 -I skill/scripts/audit_scan.py crate`: only the `CredentialHelper` name and 12 bare TODOs, not reported as findings.
- Seeded mutation harness (`probe/src/bin/fuzz.rs`): 1.5M inputs across `Diff::from_buffer`, trailers, prettify, mailmap, config, commit/tag/tree objects, revparse and name validators. It found only the diff.rs:629 panic and the binary-data abort.
- `cargo miri test`: skipped, because the unsafe paths are FFI calls into C libgit2.
- ssh and https features: not built, per the brief.

Findings: 54 (Blocker 21, Major 20, Minor 13). Last `./clock` reading: 26 minutes used of 45.
