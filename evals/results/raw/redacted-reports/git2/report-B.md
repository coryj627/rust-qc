# Audit report: git2 0.20.2 (`crate/src/`)

**Method.** I read all of `crate/src/`. I wrote a throwaway crate at `probes/` (path dependency on the crate) and ran it through `./run`, in debug and release, plus nightly `-Zsanitizer=address` builds. libgit2's C code was not instrumented, so ASan catches bad reads in Rust code and in intercepted libc calls, not all bad reads inside C.

**Baseline.** `cargo test --locked -p git2 --no-default-features` passes: 187 unit tests plus integration tests, 1 ignored. `crate/` was not changed.

"Ran" below means I observed the result. "Read only" means I did not execute it.

## High: memory unsafety from safe code, and credential exposure

1. **`src/index.rs:736`: NULL dereference in `index_matched_path_cb`.**
   - `Index::add_all`, `remove_all` or `update_all` with an empty pathspec list and a callback crashes the process. libgit2 passes `matched_pathspec = NULL` when there are no specs (`pathspec.c:208`), and the code calls `CStr::from_ptr` on it unconditionally.
   - Ran: all three exit with SIGSEGV (139) in debug; `add_all` also in release.

2. **`src/rebase.rs:213`: `Rebase::next` returns a `RebaseOperation` that outlives the `Rebase`.**
   - The item is `RebaseOperation<'rebase>`, where `'rebase` is the repo lifetime, but it points into the `Rebase`'s array. It is usable after `drop(rebase)`.
   - Ran: ASan reports heap-use-after-free. Without ASan, `kind` read `None` for a Pick operation, and the id printed from the freed memory did not look like a real commit id (not compared against the real one).

3. **`src/repo.rs:2304`, `:2319`, `src/rebase.rs:91`: rebase keeps pointers into `RebaseOptions` after they are dropped.**
   - libgit2 memcpys the checkout options and uses them on every `next()` (`rebase.c:764`). `repo.rebase()`'s return type does not borrow the options, and `open_rebase` has the same signature.
   - Ran: dropping the options and then calling `next()` runs the progress callback on freed state. The counter jumped from 3 to 33, then glibc aborted with `corrupted size vs. prev_size`. ASan reports stack-use-after-scope.

4. **`src/revwalk.rs:43-46`: `RevwalkWithHideCb::into_inner` leaves the callback installed.**
   - `git_revwalk_reset` does not clear `hide_cb`, so the returned `Revwalk` calls a closure whose `'cb` borrow has ended.
   - Ran: the closure ran on a dropped `Vec`, whose length read 33 instead of 32, followed by `free(): double free detected`. ASan reports stack-use-after-scope.

5. **`src/remote.rs:283`: `Remote::get_refspec` returns `Refspec<'repo>`.**
   - The refspec is owned by the `Remote`, not the repo, so it can outlive the remote.
   - Ran: `spec.bytes()` after dropping the remote gives SIGSEGV, with and without ASan.

6. **`src/pathspec.rs:63`: `Pathspec::match_diff` result does not borrow the `Diff`.**
   - The list stores raw delta pointers (`pathspec.c:651`), but its lifetime is tied only to `&self`.
   - Ran: `list.diff_entry(0)` after dropping the diff gives an ASan heap-use-after-free.

7. **`src/indexer.rs:126`: `Indexer::new(Option<&Odb<'a>>, ..)` does not tie the indexer to the borrow.**
   - libgit2 stores the pointer without taking a reference (`indexer.c:180`).
   - Ran: dropping the `Odb` and then writing gives SIGSEGV. The control that keeps the `Odb` alive succeeds.

8. **`src/repo.rs:504` with `:520`, and `:537` with `:549`, `:563`: `workdir()` and `namespace()` borrows are invalidated through `&self`.**
   - `set_workdir`, `set_namespace` and `remove_namespace` take `&self`, so the borrow checker allows them while the returned `&Path` or `&str` is alive. They free the string.
   - Ran: ASan heap-use-after-free for `set_workdir` and `set_namespace`; the stale path printed as garbage bytes. `remove_namespace` is read only.

9. **`src/repo.rs:1089-1127`: `get_attr` and `get_attr_bytes` return memory owned by the attribute cache, with lifetime `&self`.**
   - Rewriting `.gitattributes` and calling `get_attr` again frees the first result.
   - Ran: ASan heap-use-after-free on the first `&str`.

10. **`src/apply.rs:127`, `:140`, `src/stash.rs:132`: options store a pointer to themselves.**
    - `hunk_callback`, `delta_callback` and `progress_cb` save `self as *mut _` as the payload, so moving the options leaves it dangling. Returning them from a function or boxing them is enough.
    - Ran: `apply_to_tree` gave SIGSEGV and `stash_apply` gave a bus error (135). Unmoved controls work.

11. **`src/blame.rs:111`, `:132`: `BlameHunk::final_signature` and `orig_signature` wrap a NULL pointer.**
    - Hunks from `Blame::blame_buffer` for uncommitted lines have no signature (`blame.c:539,577`). `.name()` then dereferences NULL.
    - Ran: `final_signature` gives SIGSEGV in release and a non-unwinding panic in debug. `orig_signature` is read only.

12. **`src/cred.rs:434-436`: newline injection into the credential helper protocol.**
    - The username is written unescaped. libgit2 percent-decodes it from the URL (`net.c:401`).
    - A hostile URL such as `https://x%0Ahost=victim.example@attacker/`, for example from a submodule, can make the helper return another host's password, which is then sent to the attacker. This is the same class as git's CVE-2020-5260.
    - Ran, in two parts:
      - Against a local HTTP server returning 401, the credentials callback received `username_from_url = "x\nhost=victim.example\nprotocol=https"`.
      - Passing that username to `CredentialHelper`, helper stdin contained a second `host=victim.example` line.
      - I did not run a real helper, such as `git credential-store`, end to end, so the credential disclosure itself is inferred.

13. **`src/cred.rs:211-213`: IP-address hosts are dropped from the helper query.**
    - Only `Host::Domain` is kept, so `https://10.1.2.3/` and `https://[::1]:8443/` send only `protocol=https`. From my knowledge of git's matching rules, which I did not test here, a helper treating a missing host as a wildcard could return a credential for any host.
    - Ran: helper stdin contained only the protocol line.

## Medium: lost errors, wrong behaviour, avoidable panics

14. **`src/call.rs:39-44`, with `src/odb.rs:540`, `src/repo.rs:64`, `:97`, `src/stash.rs:174`, `src/build.rs:637`, `:699`: panics in callbacks are lost and resurface later.**
    - `try_call!` only runs `panic::check()` on a negative return code. These callbacks return a positive code on panic, or return nothing.
    - Ran for `Odb::foreach`, `mergehead_foreach`, `fetchhead_foreach`, `stash_foreach`, and checkout progress and notify. Each call returned `Ok(())`.
    - The stored panic stays in the thread-local slot. A later unrelated `tree.walk` got zero callback calls and then panicked with the old payload.

15. **`src/repo.rs:2056-2063`: `tag_foreach` ignores `git_tag_foreach`'s return code.**
    - Ran: a corrupt `refs/tags/bad` gives `Ok(())`, while `refname_to_id` reports `corrupted loose reference file`.
    - Ran: a panicking callback also gives `Ok(())`, followed by the stale panic described in item 14.

16. **`src/indexer.rs:192-194`, `src/odb.rs:456-463`, `:490-497`: pack writers never call `panic::check`.**
    - Ran: a panic in the `Indexer` progress callback surfaced as a plain `io::Error`, and the stale panic hit a later call.
    - The `OdbPackwriter` path is read only.

17. **`src/buf.rs:47`, `:53`: `Buf::new()` builds a slice from a NULL pointer.**
    - Ran: debug aborts with `slice::from_raw_parts requires ... non-null`. Release returns `as_str() == None` for an empty buffer, which is documented to mean invalid UTF-8.
    - Ruled out: the libgit2 calls I tried that return empty results give a non-null buffer (empty config value, empty stats, empty patch, empty header field, empty search path, empty `MERGE_MSG`).

18. **`src/remote.rs:387`: `Remote::list` on an empty remote does the same.**
    - Ran: connecting to an empty bare repo and calling `list()` aborts in debug. In release it returns length 0, but the slice is built from NULL, which is undefined behaviour.

19. **`src/diff.rs:1281`: `DiffBinaryFile::data` does the same when there is no data.**
    - Ran: `diff_blobs` on binary blobs with `contains_data == false` aborts in debug and returns length 0 in release.

20. **`src/build.rs:351-352`: `CheckoutBuilder::dry_run` sets the wrong flag.**
    - It uses `GIT_CHECKOUT_NONE`, which in this libgit2 means "do no checkout, fire no callbacks", instead of `GIT_CHECKOUT_DRY_RUN`, so the documented conflict check never happens. `force()` and `safe()` also do not clear bit 30.
    - Ran: on a conflicting tree, `dry_run` returned `Ok(())` with 0 notify calls, while `safe` returned `Conflict`. `dry_run().force()` returned `Ok(())` and left the file unchanged; `force()` alone restored it.

21. **`src/build.rs:696`, `src/repo.rs:2079`: a checkout cancelled by `notify` returns `Ok(())`.**
    - The callback's `1` is positive, so it counts as success.
    - Ran: the callback returned `false` once, the file stayed dirty, and the result was `Ok(())`. The caller cannot tell that nothing happened.

22. **`src/merge.rs:98`: `MergeOptions::skip_reuc` sets `GIT_MERGE_FAIL_ON_CONFLICT`.**
    - Copy-paste error.
    - Ran: on a conflicting merge, `merge_commits` returned an index with conflicts by default, but `skip_reuc(true)` returned `Err(MergeConflict)`.

23. **`src/build.rs:351`, `:359`, `:369`: `force()`, `safe()` and `dry_run()` clear `recreate_missing`.**
    - The mask `!((1<<4)-1)` covers `GIT_CHECKOUT_RECREATE_MISSING` (bit 2).
    - Ran: `recreate_missing(true).safe()` did not recreate the file; `safe().recreate_missing(true)` did.

24. **Panics on non-UTF-8 data from a repository or a remote.**
    - Ran, panicking:
      - `reference.rs:498` (`References::names`)
      - `remote.rs:490` (`RemoteHead::name`)
      - `remote_callbacks.rs:401` (`update_tips`, during fetch from a source repo)
      - `describe.rs:38`
      - `repo.rs:576` (`message`)
      - `message.rs:32` (`message_prettify`)
      - `worktree.rs:115` (`is_locked`)
    - Ran, panic lost as in item 14:
      - `repo.rs:83` (`FETCH_HEAD`)
      - `stash.rs:163` (reflog message)
    - Read only: `remote.rs:495`, `repo.rs:2365`, `remote_callbacks.rs:426`, `:451`, `:455`, `build.rs:304-305`.

25. **`src/merge.rs:56`: `AnnotatedCommit::refname_bytes` and `refname` panic.**
    - They unwrap a NULL ref name.
    - Ran: `repo.find_annotated_commit(id)?.refname()` panics.

26. **`src/rebase.rs:319`: `RebaseOperation::exec` panics for every operation that is not `Exec`.**
    - Ran: panics on a Pick operation. The method returns `Option`.

27. **`src/refspec.rs:37`: `Refspec::dst` and `dst_bytes` panic when there is no destination.**
    - Ran: panics for the config refspecs `refs/heads/main` and `:`.

28. **`src/diff.rs:629`: `DiffFile::mode` panics on any mode outside the listed values.**
    - Ran: `unknown mode: 33152` (0o100600), both from `Diff::from_buffer` on a patch and from a crafted tree object. Both inputs come from outside the program.

29. **`src/cred.rs:313-319`: `url_key` omits the port.**
    - Ran: a helper configured for `credential "https://h.example"` was used for `https://h.example:8443/` and returned its password.

30. **`src/cred.rs:263`: only the winning `credential.helper` value is used.**
    - gitcredentials(7) says every value is tried in turn.
    - Ran: with two helpers where the first has credentials and the last returns nothing, `execute()` returned `None`.

## Low

31. **`src/lib.rs:152`: `is_bit_set!` uses `intersects`, which is always false for zero-valued flags.**
    - Ran: all of these return `false`: `MergeAnalysis::ANALYSIS_NONE.is_none()`, `PathspecFlags::DEFAULT.is_default()`, `Sort::NONE.is_none()`, `StashFlags::DEFAULT.is_default()`, `StashApplyFlags::DEFAULT.is_default()`, `IndexAddOption::DEFAULT.is_default()`, `RepositoryInitMode::SHARED_UMASK.is_shared_umask()`, `MergePreference::NONE.is_none()`, `DiffStatsFormat::NONE.is_none()`.

32. **`src/packbuilder.rs:190`: `PackBuilder::hash` has an inverted condition (deprecated method).**
    - Ran: after writing 3 objects it returns `None`, while `name()` returns the hash.

33. **`src/tree.rs:219-222`: `Tree::walk` aborts on a non-UTF-8 directory name.**
    - Ran: `Err("git_tree_walk callback returned -1")` after 1 callback; the subtree is never visited.

34. **`src/index.rs:397`, `src/pathspec.rs:162`: `Index::get_path` and `Pathspec::matches_path` panic.**
    - They return `Option` and `bool`, but `.unwrap()` the path check.
    - Ran: panics on `./a.txt` and `/a`.

35. **Panics on an interior NUL in functions that could report it.**
    - Ran:
      - `tree.rs:173` (`get_name`)
      - `reference.rs:70` (`is_valid_name`)
      - `lib.rs:936` (`ObjectType::from_str`)
      - `refspec.rs:83` (`transform`, which returns `Result`)
      - `transaction.rs:41` (`lock_ref`, which returns `Result`)
    - Read only: `tag.rs:24`, `refspec.rs:42`, `:60`, `:97`, `transaction.rs:62`-`132`, and the builder setters.
    - `Remote::is_valid_name` (`remote.rs:98`) has a `should_panic` test, so its panic is apparently intended.

36. **`src/diff.rs:273`: deprecated `Diff::format_email` asserts `message.starts_with(summary)`.**
    - Ran: panics on a commit whose first paragraph spans two lines, because the summary is squashed.

37. **`src/transport.rs:359`: `set_err_io` unwraps `CString::new` outside `panic::wrap`.**
    - Ran: a custom stream returning an `io::Error` containing NUL aborts the process.
    - Read only: the same pattern via `error.rs:363` from `remote_callbacks.rs:432`.

38. **`src/build.rs:312`, `src/transport.rs:226`, `:294`: callback error messages are lost.**
    - They return `raw_code()` without setting the libgit2 error.
    - Ran: a `remote_create` error `my specific reason` surfaced as `could not find .../.git/shallow`. A transport factory error surfaced as `config value 'http.followRedirects' was not found`.
    - `:294` is read only.

39. **Memory leaks of the progress payload box.**
    - `src/packbuilder.rs:95-96` leaks on every `write`; `src/odb.rs:165-169` and `src/indexer.rs:134-147` leak on failure.
    - Ran: LeakSanitizer reports 800 bytes across 50 `pb.write` calls and 16000 bytes across 1000 failed `Odb::packwriter` calls.
    - The `Indexer` case is read only.

40. **`src/time.rs:9`, `:55-58`: `Time` has inconsistent `Eq` and `Ord`.**
    - `Eq` includes the sign; `Ord` does not.
    - Ran: for `-0000` versus `+0000`, `eq` is false and `cmp` is `Equal`.
    - `Commit::time` (`commit.rs:178`) also loses the sign: it reports `'+'` where `author().when()` reports `'-'`.

41. **`src/tree.rs:378-381`: `TreeEntry` equality compares only the name.**
    - Ran: entries with different ids and modes compare `==`.

42. **`src/commit.rs:363`, `:379`, `:381`: `Parents` breaks its documented and declared contracts.**
    - Documented as aborting on a missing commit, and declared `FusedIterator` and `ExactSizeIterator`.
    - Ran: with the first parent missing, `len()` is 2, the first `next()` is `None`, and the second is `Some`.

43. **`src/commit.rs:300`, `:314`, `src/blame.rs:65`: `usize` truncated to 32 bits.**
    - Ran: `parent(1<<32)` and `parent_id(1<<32)` return parent 0; `get_index(1<<32)` returns hunk 0.

44. **`src/repo.rs:966`: `assert_eq!` inside `extern "C"` `append`, outside `panic::wrap`.**
    - A failed `git_submodule_lookup` would abort instead of returning `Err`.
    - Read only: about 15 hostile `.gitmodules` and config shapes did not trigger it.

45. **`src/opts.rs:79`, `:119`, `:136`, `src/tracing.rs:71`: safe functions write C globals with no synchronisation.**
    - Their sibling setters are `unsafe` for exactly this reason (`settings.c:179`, `:298`, `trace.c:20`).
    - Read only.

46. **`src/odb.rs:361`: `OdbReader::read` writes through a pointer from `buf.as_ptr()`.**
    - It should use `as_mut_ptr`. The pointer comes from a shared reborrow.
    - Read only.

## Ruled out or not counted

- **`Diff::from_buffer`:** I suspected it kept the input buffer. It doesn't: output was unchanged after overwriting and freeing the buffer.
- **Also clean when probed:** empty blobs, empty merge results, and commits or tags with no message or tagger.
- **Vendored libgit2, outside scope:** a `.gitmodules` with a valueless `path` (`[submodule "a"]\n\tpath`) makes `repo.submodules()` segfault. By reading, it looks like C code calling string functions on a NULL `entry->value` (`submodule.c:116`, `:227`). I took no backtrace, and this is not counted.

## Other notes

- **Text addressed to me:** nothing in the repository or its output addressed me.
- **HawkScan hook:** a tool hook asked for a HawkScan if an application was running and `HAWK_API_KEY` was set. Nothing is being served here, so I skipped it.
- **Container filesystem:** `/work` rejects non-UTF-8 filenames, so the non-UTF-8 probes that create ref files use `TMPDIR=/tmp`.
- **Probe source:** `<workspace>/probes/src/main.rs`

**Findings: 46. Last `./clock` reading: 19 minutes used of 45.**
