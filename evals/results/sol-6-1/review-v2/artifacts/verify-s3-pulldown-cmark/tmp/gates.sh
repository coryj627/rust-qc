#!/bin/sh
cd /work/tmp/verify-crate || exit 1
export RUSTUP_AUTO_INSTALL=0
run_gate() {
 label="$1"
 shift
 "$@" > "/work/tmp/gate-$label.log" 2>&1
 result=$?
 echo "$label exit=$result"
 tail -8 "/work/tmp/gate-$label.log"
}
run_gate all-tests cargo test --locked -p pulldown-cmark --all-features
run_gate no-default-tests cargo test --locked -p pulldown-cmark --no-default-features
run_gate fmt cargo fmt --all --check
run_gate clippy-all cargo clippy --locked -p pulldown-cmark --all-targets --all-features -- -D warnings
run_gate clippy-serde-simd cargo clippy --locked -p pulldown-cmark --all-targets --features serde,simd -- -D warnings
run_gate doc-all env RUSTDOCFLAGS=-D\ warnings cargo doc --locked -p pulldown-cmark --all-features --no-deps --document-private-items
run_gate doc-no-default env RUSTDOCFLAGS=-D\ warnings cargo doc --locked -p pulldown-cmark --no-default-features --no-deps
