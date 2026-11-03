# Audit report: git2 0.20.2 (`crate/src/`)

## Summary

The crate's own tests pass, but safe code can reach undefined behaviour in many places. Most of it comes from three causes:
- Borrowed data is not tied to the object that owns it.
- Null pointers from libgit2 go straight into slices and `CStr`.
- Options structs store pointers to themselves.

Non-UTF-8 repository data, including data from a remote, panics. Panics in callbacks are often swallowed. Several option setters do something other than what their docs say.

Findings: High 21, Medium 20, Low 13, 54 in total. `crate/` was not changed. No text in the repository addressed me.

Throwaway probes are in `probe/` in the workspace. "ASan" means the probe was built with nightly `-Zsanitizer=address` and libgit2 was compiled with `-fsanitize=address`. "Debug" means the standard library's unsafe-precondition checks were active.

## High

**Credential helper input allows newline injection**
src/cred.rs:434-436 (`execute_cmd`), reached through `Cred::credential_helper`.
the username is written to the helper's stdin with `writeln!` and no newline check. A URL such as `http://a%0Ahost=evil.example@host/` gives a username containing a newline. The helper then sees a second `host=` line, and in the credential protocol the later value wins. This is the same class as CVE-2020-5260, where the helper hands over credentials for another host.

- `CredentialHelper::new("https://real.example/repo").username(Some("a\nhost=evil.example"))`: the helper's stdin was `protocol=https\nhost=real.example\nusername=a\nhost=evil.example\n`.
- A local HTTP 401 server showed libgit2 passing `username_from_url = "a\nhost=evil.example"` to the credentials callback.

**`BlameHunk` signatures dereference null**
src/blame.rs:110-112, 131-133.
hunks for lines added by `Blame::blame_buffer` have a null `final_signature` and `orig_signature`, but a non-optional `Signature` is returned. Calling `.name()` crashes.
a probe that blamed a buffer with one new line. Debug: "null pointer dereference" abort. Release: SIGSEGV.

**Crafted index causes a heap over-read**
src/index.rs:818-823 (`IndexEntry::from_raw`).
the path length is taken from the flags bits, not from the NUL-terminated string. An index file whose flags claim 1000 bytes for a 1-byte name makes `Index::get` copy adjacent heap memory into `entry.path`.
a hand-built index with a valid SHA-1. `path.len()` was 1000 and contained heap bytes. ASan: heap-buffer-overflow, READ of size 1000.

**Index callbacks crash on an empty pathspec**
src/index.rs:736 (`index_matched_path_cb`).
with an empty pathspec list, libgit2 passes a null `matched_pathspec`, and `CStr::from_ptr` dereferences it. This affects `add_all`, `update_all` and `remove_all`.
all three with `Vec::<&str>::new()` and a callback gave SIGSEGV in debug. `add_all` in release also gave SIGSEGV.

**Options structs keep a stale pointer to themselves**
src/apply.rs:127, 140; src/stash.rs:132.
`ApplyOptions::hunk_callback`, `ApplyOptions::delta_callback` and `StashApplyOptions::progress_cb` store `self as *mut _` as the payload. Moving the options afterwards, for example returning them from a function, leaves a dangling payload.
a helper function returning the options, followed by `stash_apply` or `apply`. Results were SIGBUS and SIGSEGV. ASan: stack-use-after-return on both.

**`RevwalkWithHideCb::into_inner` keeps the dangling callback**
src/revwalk.rs:43-46.
`git_revwalk_reset` does not clear the hide callback, and the returned `Revwalk<'repo>` is not bound to `'cb`. Walking after the closure is gone calls it through a dead pointer.
the closure saw garbage captures (`[2, 197, 196, 107]` instead of `[1, 1, 1, 1]`). ASan: stack-use-after-scope.

**`Remote::get_refspec` outlives the remote**
src/remote.rs:283.
it returns `Refspec<'repo>`, but the refspec is owned by the `Remote`.
reading `.str()` after dropping the remote gave SIGSEGV. ASan: heap-use-after-free.

**`RebaseOperation` outlives the `Rebase`**
src/rebase.rs:217-224.
`Iterator::Item` is `RebaseOperation<'rebase>`, where `'rebase` is the repository lifetime. The operation array is freed with the `Rebase`.
reading the operation after dropping the rebase returned a bogus `kind() == None`. ASan: heap-use-after-free.

**`set_workdir` and `set_namespace` free strings still borrowed**
src/repo.rs:504 with 520; 537-546 with 549-568.
`workdir()` and `namespace()` return borrows from `&self`, and the setters also take `&self` and free the old string.

- `namespace` silently changed to the new value.
- `workdir` printed garbage bytes.
- ASan: heap-use-after-free on both.

**`get_attr` returns data freed by the next lookup**
src/repo.rs:1089-1128.
the returned `&str` or `&[u8]` lives in the attribute cache. Another `&self` call after `.gitattributes` changes reloads the file and frees that memory.
ASan heap-use-after-free, READ of size 16, reading the first value after the second `get_attr`.

**`Indexer` does not borrow its `Odb`**
src/indexer.rs:126.
`Indexer<'a>` ties `'a` to the `Odb`'s parameter, not to the `&Odb` borrow, and libgit2 stores the pointer without a reference count.
an indexer built inside a block with `Odb::new()`, then written with verify on, gave SIGSEGV. ASan: heap-use-after-free.

**`Pathspec::match_diff` list outlives the diff**
src/pathspec.rs:63-79 (read through `diff_entry`).
the list stores pointers to diff deltas, but its lifetime is tied only to the `Pathspec`.
ASan heap-use-after-free on `list.diff_entry(0)` after the diff was dropped.

**`Reflog` outlives its repository**
src/reflog.rs:11; src/repo.rs:2638.
`Reflog` has no lifetime, but its refdb backend keeps a raw repository pointer.
`log.write()` after the repository was dropped. ASan: heap-use-after-free.

**Calling `OdbWriter::finalize` twice crashes**
src/odb.rs:390-398.
it takes `&mut self`, so a second call is safe code, and it reuses the finished stream.
the second `finalize` on a loose-backed stream gave SIGSEGV, also under ASan at a null pc.

**`Buf` deref on an empty buffer is undefined behaviour**
src/buf.rs:47, 53.
`Buf::new()` has a null `ptr`, and `Deref` passes it to `slice::from_raw_parts`.
`Buf::new().len()` aborts in debug with an unsafe-precondition violation. Release returned 0, which is silent undefined behaviour.

**`Remote::list` on an empty remote is undefined behaviour**
src/remote.rs:387.
a null base pointer with size 0 goes into `from_raw_parts`.
connecting to an empty bare repository and calling `list()` aborts in debug. Release returned `Ok(0)`.

**`DiffBinaryFile::data` is undefined behaviour without data**
src/diff.rs:1281.
`data` is null for ordinary binary diffs that lack `show_binary`, and also for a parsed patch with an empty `literal 5` body.
`Diff::foreach` with a binary callback aborts in debug, and the harness hit the same abort from a patch input. Release returned 0.

**`stash_foreach` crashes on a reflog entry without a message**
src/stash.rs:163.
`CStr::from_ptr(message)` is called on null.
removing the message from `logs/refs/stash` gave SIGSEGV.

**Non-UTF-8 ref names from a remote panic the client**
src/remote.rs:490, 495; src/remote_callbacks.rs:401, 451, 455, 426.
`RemoteHead::name`, `symref_target` and the `update_tips`, `push_update_reference` and certificate callbacks all call `from_utf8(..).unwrap()` on remote-controlled bytes.

- A local remote with branch `caf\xe9` panicked `head.name()` at remote.rs:490.
- The same remote panicked `fetch` with `update_tips` at remote_callbacks.rs:401.
- Lines 451, 455, 426 and remote.rs:495 were only read.

**`DiffFile::mode` panics on modes from patches and trees**
src/diff.rs:629.
unknown modes reach `panic!`, undocumented. `Diff::from_buffer` accepts any octal mode, and a tree object can carry unusual ones.

- `Diff::from_buffer(b"diff --git a/f b/f\nold mode 100600\nnew mode 100777\n")` panicked with `unknown mode: 33152`.
- The harness also reached this panic through a tree diff.

**Non-UTF-8 local repository data panics**
src/reference.rs:498; src/describe.rs:38; src/worktree.rs:115; src/repo.rs:576, 83, 2365; src/stash.rs:163; src/message.rs:32.
these sites call `unwrap` on `from_utf8` for repository data or caller bytes.
Established by, each panicking at the line shown:
- `references().names()` with a `caf\xe9` ref.
- `Describe::format` with a `caf\xe9` tag.
- `Worktree::is_locked` with a Latin-1 lock reason.
- `Repository::message` with a Latin-1 `MERGE_MSG`.
- `stash_foreach` and `fetchhead_foreach` with Latin-1 text.
- `message_prettify(&b"caf\xe9\n"[..], None)`.
repo.rs:2365 (`note_default_ref`) was only read.

## Medium

**Callback panics are swallowed and resurface later**
src/odb.rs:540, 456-463, 490-499; src/repo.rs:64, 97; src/stash.rs:174; src/build.rs:699; src/indexer.rs:192-197.
`try_call!` calls `panic::check` only when the return value is negative, and these paths return a positive value or skip `try_call!` altogether. The call returns `Ok(())`. Later callbacks on that thread are skipped because `panic::wrap` refuses to run them.

- `Odb::foreach(|_| panic!())` returned `Ok(())`.
- A second foreach visited 0 objects.
- The process then exited silently at an unrelated failing `Repository::open`.
- `stash_foreach`, a checkout notify panic and `OdbPackwriter::write` behaved the same way.

**`tag_foreach` ignores the libgit2 result**
src/repo.rs:2057-2063.
the return value is discarded and `Ok(())` is always returned, so errors are lost.
a panicking callback gave `Ok(())`, and a second call visited 0 tags.

**`fetchhead_foreach` drops valid `FETCH_HEAD` files**
src/repo.rs:79-80.
`assert!(!ref_name.is_null())` fires for lines libgit2 parses with no ref name. The panic is then swallowed.
`<oid>\n`, `<oid>\t\t<url>` and `remote-tracking branch 'o/m' of url` each returned `Ok(())` with 0 entries. From memory, not checked here, `<oid>\t\t<url>` is the form `git fetch <url>` writes.

**`Tree::walk` aborts on a non-UTF-8 directory**
src/tree.rs:219-222.
a root that is not UTF-8 returns -1 and aborts the whole walk, with no error set.
a tree with `caf\xe9/f` and `g` stopped before `g`, and the error was a stale, unrelated `reference 'refs/heads/master' not found`.

**Cancelling a checkout from `notify` still returns `Ok`**
src/build.rs:693-699; src/repo.rs:2128.
`false` maps to 1, which `try_call!` treats as success, although the docs say the checkout is cancelled.
`checkout_tree` with `notify` returning false gave `Ok(())` and the file was left unchanged.

**`MergeOptions::skip_reuc` sets the wrong flag**
src/merge.rs:98.
it sets `GIT_MERGE_FAIL_ON_CONFLICT` instead of `GIT_MERGE_SKIP_REUC`.
a conflicted `merge_commits` returned `Ok` with conflicts by default, and `Err(MergeConflict)` with `skip_reuc(true)`.

**`CheckoutBuilder::dry_run` does not check anything**
src/build.rs:350-354.
it sets `GIT_CHECKOUT_NONE`, not `GIT_CHECKOUT_DRY_RUN`. The docs promise conflict checking. The flag also survives later calls, because the mask does not clear bit 30.

- A safe checkout gave `Err(Conflict)`.
- `dry_run()` on the same state gave `Ok(())`.
- `dry_run().force()` gave `Ok(())` and changed nothing.

**`force()` and `safe()` clear `recreate_missing`**
src/build.rs:351, 359, 369.
the mask `(1 << 4) - 1` includes `GIT_CHECKOUT_RECREATE_MISSING` (bit 2).
`recreate_missing(true).safe()` left the file missing, and `safe().recreate_missing(true)` restored it.

**`PackBuilder::hash` has inverted logic**
src/packbuilder.rs:190.
it returns a hash only when `object_count() == 0`.
a written pack with 3 objects gave `hash=None`.

**`RebaseOperation::exec` panics on every non-exec operation**
src/rebase.rs:319.
`opt_bytes(..).unwrap()` is called on a null pointer, although the method returns `Option`.
`op.exec()` on a `Pick` panicked.

**`AnnotatedCommit::refname` panics when there is no ref**
src/merge.rs:56.
`find_annotated_commit` leaves the ref name null. The docs say `None` means only invalid UTF-8.
`refname()` panicked.

**`Diff::format_email` panics on multi-line summaries**
src/diff.rs:273, also 267, 268, 271.
the summary joins the first paragraph's lines, so `message.starts_with(summary)` fails.
the message `"line one\nline two\n\nbody\n"` panicked at line 273. Lines 267, 268 and 271 were only read.

**`CredentialHelper` drops IP-address hosts**
src/cred.rs:211.
only `Host::Domain` is kept, so `host=` is omitted and URL-scoped config is skipped.
`https://10.0.0.1:8443/repo` gave helper stdin of `protocol=https\n`.

**`CredentialHelper` reads only one `credential.helper`**
src/cred.rs:256-265.
the comment says "all", and from memory gitcredentials(7) says each configured helper is tried in turn, but `get_string` returns only the last value.
a working helper followed by a failing one gave `None`.

**`Index::get_path` and `Pathspec::matches_path` panic on ordinary paths**
src/index.rs:397; src/pathspec.rs:162.
they call `path_to_repo_path(path).unwrap()`, although the return types are `Option` and `bool`.
`get_path("")`, `get_path("/etc/passwd")` and `matches_path("")` panicked.

**Interior NUL bytes panic instead of returning an error**

- Validators and matchers: reference.rs:70; remote.rs:98; tag.rs:24; tree.rs:173; refspec.rs:42, 60; lib.rs:936.
- Functions returning `Result`: transaction.rs:41, 62-63, 90-92, 116, 132; refspec.rs:83, 97.
- Builders: build.rs:178, 532, 541, 547, 553, 559, 721, 741; diff.rs:906, 914, 920; status.rs (`pathspec`); stash.rs:49; merge.rs:237, 251, 265; remote.rs:591, 691, 701; repo.rs:3484-3519; rebase.rs:64; describe.rs:99, 162; email.rs:97; proxy_options.rs:35.
none of these panics is documented. A `#[should_panic]` test on `Reference::is_valid_name` records the behaviour.
`is_valid_name` on all three types, `Tree::get_name` and `Transaction::lock_ref` panicked. The others were only read.

**Errors from callbacks lose their message**
src/transport.rs:226, 294; src/build.rs:312.
only the code is returned; `raw_set_git_error` is not called.

- A transport factory's "factory says no" became `Err("no error")`.
- `remote_create`'s "create says no" became an unrelated `.git/shallow` stat error.
- transport.rs:294 was only read.

**A NUL in error text aborts the process**
src/transport.rs:359; src/remote_callbacks.rs:432; src/error.rs:363.
`CString::new(..).unwrap()` runs outside `panic::wrap`, inside `extern "C"`.
a custom transport stream returning `io::Error` with `"bad\0byte"` aborted with "panic in a function that cannot unwind". The certificate path was only read.

**Safe global setters race with other threads**
src/opts.rs:79, 119, 136 (`enable_caching`, `strict_object_creation`, `strict_hash_verification`).
they write plain `bool` globals without synchronisation, such as `git_cache__enabled`, read elsewhere with no lock. `set_cache_object_limit` is `unsafe` for this same reason.
reading the code only. A ThreadSanitizer run did not flag these writes.

**`execute_cmd` panics on a blank helper when `sh` is missing**
src/cred.rs:392.
`parts.next().unwrap()` runs on an empty command.
`credential.helper = !` with `PATH=/nonexistent` panicked.

## Low

**`PackBuilder::write` leaks on every call**
src/packbuilder.rs:95-96; error paths at src/odb.rs:165-175 and src/indexer.rs:134-150.
the `Box::into_raw` payload is never freed.
a counting allocator showed live allocations grew by exactly 2000 after 2000 calls. The error paths were only read.

**Narrowing casts return the wrong element**
src/commit.rs:300, 314; src/blame.rs:65.
`usize` is cast to `u32` with truncation.
`commit.parent(1 << 32)` and `parent_id(1 << 32)` returned parent 0. blame.rs:65 was only read.

**`Parents` and `ParentIds` do not abort as documented**
src/commit.rs:362-363 and 386-389 (the `and_then` iterator bodies).
the docs say iteration aborts on a missing commit, and the type claims `FusedIterator`. It yields `None` and then `Some`.
with a missing first parent, `len` was 2, first `None`, second `Some`.

**`IndexEntries` panics when another handle shrinks the index**
src/index.rs:761.
`repo.index()` returns handles to one shared index.
`a.iter()`, then `b.clear()`, then `next()` panicked.

**`Time` equality and ordering disagree, and `Commit::time` loses the sign**
src/time.rs:9, 57; src/commit.rs:178.
`Eq` includes `sign` and `Ord` does not. `Commit::time` also rebuilds the time without `-0000`.
`time.sign` was `+`, `when.sign` was `-`, `eq` false, `cmp` Equal.

**Writes go through pointers derived from shared references**
src/odb.rs:361; src/buf.rs:63; src/message.rs:141; src/call.rs:78.
`buf.as_ptr() as *mut` and `&self.raw as *const _ as *mut _` are then written by C. This breaks the aliasing rules, though I saw no misbehaviour.
reading the code only. Miri cannot run these paths.

**`Send` for `Diff` and `Patch` has no justification**
src/diff.rs:25; src/patch.rs:18.
both hold repository pointers while `Repository` is `!Sync`, so another thread can use the repository concurrently.
reading the code only. A 4-thread stress under ASan found no crash.

**Casts rely on `repr(Rust)` layout**
src/remote.rs:388; src/remote_callbacks.rs:519; src/object.rs:151, 161; src/cert.rs:97.
slices and references are reinterpreted between types without `repr(transparent)`.
reading the code only.

**Panic can abort inside `extern "C"` in `submodules`**
src/repo.rs:966.
`assert_eq!(rc, 0)` runs with no `panic::wrap`.
reading the code only. Ten `.gitmodules` shapes did not trigger it.

**Undocumented unsafe**
crate-wide.
798 unsafe blocks and 8 `unsafe impl`s have no SAFETY comment, and 17 `unsafe fn`s have no `# Safety` section.
advisory clippy run, plus a grep finding SAFETY only in tracing.rs.

**Missing `# Errors` and `# Panics` sections**
crate-wide.
428 functions lack `# Errors` and 117 lack `# Panics`.
advisory clippy run.

**Missing `Debug` implementations**
crate-wide.
113 public types do not implement `Debug`.
advisory clippy run.

**Lint configuration and toolchain drift**
src/lib.rs:69 and crate-wide.

- `cargo clippy -D warnings` fails on Rust 1.99.
- From the CI config, its rustfmt job runs `cargo fmt`, and my grep of the workflows found no clippy step.
- 16 `#[allow]`s have no reason.
running clippy and reading `.github/workflows/main.yml`.
