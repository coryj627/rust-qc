# Author-only provenance

- Case ID: `wikilink`; bug ID: `pulldown-cmark-3` (the upstream test comment additionally identifies issue #1108).
- Repository: `pulldown-cmark_pulldown-cmark.git`.
- Baseline supplied at `/work/crate`: pulldown-cmark 0.13.0. No baseline commit ID was supplied.
- Exact upstream fixed snapshot supplied at `/work/fixed`, commit `ebf31da886d90f34d9519278e323dbefb08444af` (identity from case-input.json; snapshots have no Git metadata).
- Oracle source: `pulldown-cmark/src/parse.rs::test::wikilink_malformed_no_subtract_overflow`, lines 3007–3019 in the supplied fixed snapshot.
- `upstream-original.rs` is an exact byte extraction of that attributed function, including its cfg, comments, input, rendering, and assertion.
- `case_wikilink_oracle.rs` prepends only imports and a public re-export of `pulldown_cmark::html`. This permits the unchanged `crate::html::push_html` expression to resolve in an integration test. The test body, `Options::all()`, and nonempty-output assertion are unchanged. No production edits or dependencies are required by this adaptation.
- `case_wikilink_neighbors.rs` contains independently authored task-derived tests. They exercise malformed delimiters with Unicode and wikilinks enabled, valid wikilinks/images, ordinary Markdown, and disabled wikilinks. They do not impose an exact rendering for malformed inputs.
- All material in this directory is withheld until after the candidate returns. No treatment outputs or arm mappings were used.
