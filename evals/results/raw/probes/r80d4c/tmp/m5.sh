cd /work/tmp/aba && cargo run --offline 2>&1 | grep -vE "Compiling|Finished|Running|warning|^ *(\||=|-->|[0-9]+ \|)|^$" | cut -c1-200
cd /work/crate
FEATS=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes
echo "== clippy"; cargo clippy --all-targets --features $FEATS -- -D warnings 2>&1 | grep -E "^(error|warning)" | sort | uniq -c | sort -rn | cut -c1-140 | head -20
echo "== audit lints"; touch src/lib.rs; cargo clippy --lib --features $FEATS -- -W missing_docs -W missing_debug_implementations -W clippy::missing_errors_doc -W clippy::missing_panics_doc -W clippy::undocumented_unsafe_blocks -W clippy::unwrap_used -W clippy::allow_attributes_without_reason -W clippy::missing_assert_message 2>&1 | grep -E "^warning" | sed -E 's/`[^`]*`/X/g' | sort | uniq -c | sort -rn | cut -c1-110 | head -14
for f in "" "--no-default-features" "--features serde" "--features ufmt" "--features defmt" "--features bytes" "--features alloc" "--features portable-atomic" "--features mpmc_large" ; do printf '%s: ' "check $f"; cargo check --offline $f 2>&1 | grep -E "^error|Finished" | head -1 | cut -c1-120; done
