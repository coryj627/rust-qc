# Audit of just 1.50.0 (crate/src)

Verdict: the core runner works and its unit tests pass, but `just --fmt` silently changes or deletes content when it rewrites a justfile. There are also many panics and wrong results on legitimate input, especially around modules, `set lazy`, `set ignore-comments`, user-defined functions and the dotenv settings. 

Unless a finding says otherwise, I established it by running `/target/debug/just` (built with `cargo build --locked --bin just`) in the container on the input shown. I did not change `crate/`; harnesses and probes are in `tmp/`. No repository text I read addressed me as an agent. `crate/skills/` was not read.

## High

**--fmt drops the `eager` keyword**
src/assignment.rs:6-18 (Display for Assignment).
`eager z := "3"` is rewritten to `z := "3"`. Under `set lazy` the variable then stops being evaluated, so running `--fmt` changes behaviour. Found by comparing `--dump --dump-format json` before and after a dump round trip (tmp/rt.py, about 3400 generated justfiles), then confirmed with `just --fmt`.

**--fmt drops `[private]` on aliases and modules, and bare `[doc]` on modules**
src/alias.rs:40-43; src/item.rs:59-90 (the Module arm ignores `private`, and `doc: None` is lost).
`[private]\nalias a := foo` and `[private]\nmod m` lose the attribute, so hidden items become public in `--list`. `[doc]\nmod n "n.just"` loses its suppression. Shown by a `diff` of the file before and after `just --fmt`.

**--fmt deletes comments on recipe headers and attribute lines**
src/parser.rs:228-236 (`expect_eol`), called from parser.rs:1231 and parser.rs:1588.
`qux: # comment before body\n  echo qux` becomes `qux:\n    echo qux`. `foo: bar # depcomment` followed by a body also loses the comment. `[private] # c` loses it under `--dump`, so the same printer loses it for `--fmt`. The text is gone from the user's file. Shown by `diff` after `just --fmt`.

**--fmt makes a multi-line module doc unparsable**
src/item.rs:68.
`[doc("a\nb")]\nmod? m` dumps as `# a\nb\nmod? m`. Re-reading that output fails with `Expected '*', ':', '$', identifier, or '+', but found end of line`. `--fmt` shares this printer, so it would write a justfile that no longer compiles.

## Medium

**A first-line shebang is taken as a doc comment and corrupted**
src/parser.rs:381-388 (`take_doc_comment`).
`#!/usr/bin/env just --justfile\nfoo:\n  echo hi` lists as `foo # !/usr/bin/env just --justfile`. `--dump` prints `# !/usr/bin/env just --justfile`, so `--fmt` breaks the executable script.

**Recursive user functions overflow the stack**
src/assignment_resolver.rs:27-31 (function bodies get only an arity check); src/evaluator.rs:219-253.
both `f(a) := f(a)` and `x := f("a")` with `f(a) := x` pass analysis, then abort at run time with `fatal runtime error: stack overflow`, exit 134.

**User functions evaluate in a fresh context**
src/evaluator.rs:226 (root scope), 241 (empty overrides), 246 (`is_dependency: false`).

- With `x := "orig"` and `f(a) := a + x`, `just x=new r` gives `orig` from `f` and `new` from `{{x}}` in the same line.
- A variable's backtick reruns on every function call: three runs where one is expected.
- A backtick in a function body does not see `export E` (prints `unset`, while a direct backtick prints `exp`).
- `is_dependency()` inside a function is always `false`.

**Invalid strftime formats panic**
src/function.rs:246 and 250 (`datetime`, `datetime_utc`); src/config.rs:139 (`--timestamp-format`).
`datetime("%Q")`, `datetime_utc("%")` and `just --timestamp --timestamp-format '%Q' r` panic with `a Display implementation returned an error unexpectedly`, exit 101.

**Ambiguous module outside the parent directory panics**
src/compiler.rs:204.
with `mod foo "../bar"` where `../bar` contains both `justfile` and `.justfile`, `just --list` panics with `StripPrefixError`. An absolute or `~/` path should reach the same code; I ran only the `../` case.

**Trailing backslash on an indented last line panics the lexer**
src/lexer.rs:780-783 (`lex_escape` accepts `\` at EOF without a token), then the assert at lexer.rs:610.
the 2-byte file `\t\` panics with `assertion left == right failed, left: 1, right: 0`, exit 101. `x := "a"\n\t\` does the same. Found by tmp/fz, an in-process mutation fuzzer through `just::run` (10.2M runs across `--list`, `--dump`, `--summary`, `--fmt --check` and others). This was its only panic.

**set ignore-comments hides lines that are later evaluated**
src/unresolved_recipe.rs:61 versus src/recipe.rs:442-444 (scripts) and recipe.rs:286-301 (continuations); the panic is at src/evaluator.rs:358.
resolution skips any comment-looking line, but script recipes and continued lines still evaluate them.
- `set ignore-comments`, `r:`, `#!/bin/sh`, `# {{ nofn() }}` panics with `entered unreachable code`.
- `# {{nope}}` gives an Internal runtime error.
- The same happens for `echo a \` followed by `# {{nofn()}}`.
- With `set lazy`, the valid `#!{{i}}` and `[script]` `# {{v}}` fail with `attempted to evaluate undefined variable`.

**--fmt passes the wrong root to the loader**
src/subcommand.rs:352, which reaches src/loader.rs:29.
`format` passes the justfile's directory where the justfile path is expected. `--justfile /.dockerenv --fmt --check` panics at `root.parent().unwrap()`, so any justfile directly in `/` should panic the same way. I tested only with that existing root file, because `/` is read-only here. Elsewhere, fmt errors show `w/justfile:1:4` instead of `justfile:1:4`.

**Stdout failures panic**
print macros throughout src/subcommand.rs, and src/justfile.rs:307-328.
`just --list | head -1` on a 20000-recipe file panics with `failed printing to stdout: Broken pipe`, exit 101. `--list`, `--evaluate`, `--summary` and `--dump` with stdout on `/dev/full` panic with `No space left on device`.

**A file sharing a module's name breaks module lookup**
src/compiler.rs:158-175; src/filesystem.rs:7 (import case).
with `mod build`, `build.just` and a regular file `build` (a script or binary), the result is `I/O error reading directory .../build: Not a directory`, even though `build.just` exists. `mod? build` with only the file `build` fails the same way, as do `mod? a "x/y"` and `import? "x/y"` when `x` is a file.

**Two modules from one file share one AST**
src/compiler.rs:19 (the AST is keyed by file path, and later visits are skipped).
with `mod a "x.just"` and `mod b "x.just"`:
- `just a::r` reports `module_path()` as `b`.
- `--summary` prints `b::r b::r`.
- `just a::r b::r` runs the recipe only once.
- `a::v=9 a::r` silently ignores the override, while `b::v=9 a::r` applies it.
Separately, if `a.just` and `b.just` both `import "common.just"`, then `a::v=9 b::r` prints 9, because assignment Numbers are shared.

**A missing dotenv-path silently loads another file**
src/load_dotenv.rs:27-41.
with `set dotenv-path := "missing.env"` and an unrelated `.env` present, the `.env` values are loaded. `set dotenv-required` is satisfied by that wrong file. The settings table (README:1039) says "error if not present", and the prose says the path is only checked relative to the working directory. An existing test does require a directory at the path to fall through (`dotenv_path_does_not_override_dotenv_file`), but nothing documents this for a missing file.

**Dotenv ignores set working-directory**
src/justfile.rs:111-116.
with `set working-directory := "sub"` and `set dotenv-path := ".env"`, `pwd` is `.../sub` but the value comes from the root `.env`. `set dotenv-load` with only `sub/.env` finds nothing. The README says the path is relative to the working directory.

**[env] loses to exports and dotenv**
src/recipe.rs:362-368 and 527-538 (`[env]` is applied before `export`).
`export FOO := "exported"` with `[env("FOO","attr")]` prints `exported`, in linewise and shebang recipes. With `set dotenv-load`, the dotenv value wins as well. The most specific setting loses.

**--dry-run executes shell()**
src/function.rs:548-557.
`x := shell("touch ran; echo v")` with `just --dry-run r` creates `ran`, while a backtick beside it stays unrun and prints literally.

**--evaluate-format shell corrupts `!`**
src/justfile.rs:323.
`!` is escaped inside double quotes, where the backslash is kept. Value `a!b` is emitted as `x="a\!b"`, and sourcing it in sh or bash yields `a\!b`.

**A failed shared dependency reruns under [parallel]**
src/justfile.rs:429-433 and 489 (`*guard` is set only on success).
`[parallel] a: b c`, `b: x`, `c: x`, with `x` failing, prints `running-x` twice.

**set lazy is exponential on dependency diamonds**
src/justfile.rs:190-201 (no visited set).
a chain of 26 diamonds (`a{i}: b{i} c{i}`, both depending on `a{i+1}`) was still running when killed at 30s. The same file without `set lazy` takes 5ms.

**Unused constant asserts break everything**
src/evaluator.rs:37-43 (`evaluate_settings` evaluates every assignment).
`x := assert("a" == "b", "unused assert")` makes `just --list`, `--dump` and an unrelated `just r` fail, even under `set lazy`. A bad regex in an unused `=~` also fails `--list`.

**Alias to a submodule recipe is credited to a local recipe**
src/subcommand.rs:557-558.
with `mod m`, `alias x := m::foo` and a local `foo`, `--list` shows `foo # [alias: x]`, but `just x` runs `m::foo`. By reading the code, with no local `foo` the alias is not listed at all.

**Empty chooser output runs the default recipe**
src/subcommand.rs:283-290.
`just --choose --chooser true` runs the first recipe (`chosen-a`) although nothing was chosen.

**--choose offers recipes from private modules**
src/subcommand.rs:232.
`[private] mod p`'s recipe reaches the chooser and runs, though `--list` and `--summary` hide it.

**A relative --ceiling is silently ignored**
src/config.rs:295; src/search.rs:196-197 and 246-247.
from `sub/`, `--ceiling .`, `--ceiling ../sub` and `JUST_CEILING=.` still find the parent justfile. An absolute path correctly stops the search.

**is_dependency() is always true in dependency arguments and confirm prompts**
src/justfile.rs:457.
`a: (b is_dependency())`, run as `just a`, passes `true` while `a`'s body prints `false`. `[confirm(is_dependency())]` on a top-level recipe prompts `true`.

**Indented format strings unindent after evaluation**
src/evaluator.rs:385-387.
with `v := "  deep\n shallow"`, `f'''\n    x\n    {{ v }}\n'''` keeps the literal indentation because the interpolated value lowers the common indent. `f"""\n\t  a\n\t  b\n"""` with escaped tabs returns `a\nb\n`, while the same plain `"""` string returns `\t  a\n\t  b\n`.

**Failure line numbers are wrong after multi-line interpolations**
src/recipe.rs:274 and 291 (counts Line items).
with `r:\n  echo {{\n    "a"\n  }}\n  false\n`, the failure is reported on line 3 but `false` is on line 5.

**A module's set export exports its parent's variables**
src/command_ext.rs:52.
a root `_secret := "x"` and non-exported `plain := "y"` both reach the environment of a recipe in a module with `set export` (prints `x y`).

**--command loses the child's exit code**
src/error.rs:270-288 (`code()` has no `CommandStatus` arm).
`just --command sh -c 'exit 7'` exits 1. `EditorStatus` and `ChooserStatus` propagate their status.

## Low

**set lazy in a module is ignored**
src/justfile.rs:187.
only the root setting is read, so a module's `set lazy` still evaluates its unused backtick.

**Even trailing backslashes count as continuation**
src/line.rs:46-51.
`echo a\\` then `echo b` becomes `echo a\echo b` and prints `aecho b`.

**--init creates a conflicting justfile**
src/subcommand.rs:410.
with an existing `.justfile` or `Justfile`, `--init` writes `justfile`. After that every command fails with `Multiple candidate justfiles found`.

**Directories are accepted as justfile candidates**
src/search.rs:162-185; src/compiler.rs:172-190.
`sub/justfile/` stops the upward search with `Is a directory` even though the parent has a justfile. `build/justfile/` breaks `mod build` the same way.

**Option defaults falsely trigger the required-parameter error**
src/analyzer.rs:371.
`[arg("o", long)] r o="d" p:` is rejected with `Non-default parameter p follows default parameter`, though an option does not affect positional order.

**Duplicate function parameters are accepted**
src/parser.rs:605-610.
`f(a, a) := a` compiles and the last argument wins.

**Module circular-import check misses unclean paths**
src/compiler.rs:125-128.
`mod x "../w/justfile"` and `mod x "sub/../justfile"` recurse until a name-too-long I/O error on a very long path, instead of `CircularImport`. A symlink cycle through `import` behaves the same.

**SIGTERM can target a reaped PID**
src/signal_handler.rs:135-139.
the child is waited on before it is removed from `children`, so a signal in that window can hit a reused PID. Established by reading the code only.

**A guard line exiting 1 ignores a caught signal**
src/recipe.rs:376-379.
after SIGTERM, a guard command exiting 1 returns `Ok`, so the dependent's next command (`after-r`) runs before `Interrupted` is reported.

**Editor and chooser are not under the signal guard**
src/subcommand.rs:334 (editor) and subcommand.rs:~250 (chooser spawn).
SIGINT to `just --edit` makes `just` exit 130 while the editor keeps running. SIGTERM during `--choose` does the same to the chooser.

**JSON dump order of unexports is unstable**
src/justfile.rs:33.
six runs of `--dump --dump-format json` gave six different orders of `unexports`.

**Resolved alias display loses the module path**
src/alias.rs:46-53; also the JSON `target` field.
`--show x` prints `alias x := foo` for `alias x := m::foo`. JSON gives `target: foo`, which is a different recipe if a local `foo` exists.

**Empty evaluated shebang is an internal error**
src/recipe.rs:486.
`#!{{ "" }}` reports `Internal runtime error ... bad shebang line`.

**Mixed-whitespace check in recipe bodies is dead**
src/lexer.rs:351-355.
`.take(n).map(i).next()` always yields 0, so `body_whitespace` is always empty. `a:\n  b\n \tc` reports inconsistent whitespace, never mixed.

**Very long chains overflow the stack**
src/assignment_resolver.rs:38-57; src/recipe_resolver.rs:39-74.
60000 chained assignments or 30000 chained recipes abort with a stack overflow. This was measured on a debug build; release limits will differ.

src/alias.rs:17; src/invocation_parser.rs:333, 404, 648.
the only errors are the assert-style lints, seen on 1.99.1. The project pins no toolchain, so I treat this as toolchain drift.

**Rustdoc and advisory lint results**
`RUSTDOCFLAGS="-D warnings" cargo doc --document-private-items` fails on an unresolved link and footnote references. The advisory pass reports 203 lib warnings, mostly `use super::*` (the project's convention) plus some missing docs.

## Low

**VISUAL and EDITOR cannot carry arguments**
src/subcommand.rs:334.
`VISUAL="sh -c true" just --edit` fails with `No such file or directory`, so common values like `code --wait` will not work. Consider running the value through a shell.
