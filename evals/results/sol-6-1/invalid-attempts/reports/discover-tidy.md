The workspace is read-only, so I couldn’t apply the changes. This patch centralizes the settings while preserving names, versions, and runtime behavior:

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -1,3 +1,10 @@
 [workspace]
 members = ["alpha", "beta"]
 resolver = "2"
+
+[workspace.package]
+edition = "2021"
+license = "MIT"
+
+[workspace.lints.rust]
+unsafe_code = "forbid"
--- a/alpha/Cargo.toml
+++ b/alpha/Cargo.toml
@@ -1,8 +1,8 @@
 [package]
 name = "alpha"
 version = "0.1.0"
-edition = "2021"
-license = "MIT"
+edition.workspace = true
+license.workspace = true
 
-[lints.rust]
-unsafe_code = "forbid"
+[lints]
+workspace = true
--- a/beta/Cargo.toml
+++ b/beta/Cargo.toml
@@ -1,8 +1,8 @@
 [package]
 name = "beta"
 version = "0.2.0"
-edition = "2021"
-license = "MIT"
+edition.workspace = true
+license.workspace = true
 
-[lints.rust]
-unsafe_code = "forbid"
+[lints]
+workspace = true
```

Baseline formatting, Clippy, documentation, and tests passed. The proposed patch remains unapplied and unverified.