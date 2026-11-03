cd /work/crate
F=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes
export CARGO_TARGET_DIR=/target/h
echo == clippy default rules; cargo clippy --offline --locked --all-targets --features $F -- -D warnings 2>&1 | grep -E "^(error|warning)" | sort | uniq -c | sort -rn | head -14
cargo clippy --offline --locked --all-targets --features $F -- -D warnings 2>&1 | grep -E "^\s+--> " | head -14
echo == audit lints; cargo clippy --offline --locked --lib --features $F -- -W missing_docs -W missing_debug_implementations -W clippy::missing_errors_doc -W clippy::missing_panics_doc -W clippy::undocumented_unsafe_blocks -W clippy::unwrap_used -W clippy::allow_attributes_without_reason -W clippy::wildcard_imports -W clippy::missing_assert_message -W clippy::map_err_ignore -W clippy::missing_safety_doc 2>&1 | grep -E "^warning" | sed -E 's/`[^`]*`/X/g' | sort | uniq -c | sort -rn | head -14
echo == per feature
for f in "" alloc bytes defmt mpmc_large serde ufmt portable-atomic portable-atomic-critical-section nightly; do
  if cargo check --offline --locked --features "$f" >/tmp/o 2>&1; then echo "ok   [$f]"; else echo "FAIL [$f]"; grep -E "^error" /tmp/o | head -3; fi
done
