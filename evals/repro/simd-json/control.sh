#!/bin/sh
# usage: control.sh <bug-id> <fix-short-sha> <mode> [args]  mode: native | native-release | miri-x86 | miri-x86-release | miri-aarch64
# Builds the bug's reproducer against /work/fix/<sha> instead of the baseline and prints what happens.
id=$1; sha=$2; mode=$3
export TMPDIR=/target/tmp; mkdir -p $TMPDIR
C=/target/ctl/$id-$sha; mkdir -p $C/src
F=/target/nolint-fix-$sha; mkdir -p $F
cp -r /work/fix/$sha/. $F/ && sed -i '1{/deny(warnings)/d}' $F/src/lib.rs
cp /work/repro/$id/src/main.rs $C/src/main.rs
sed -e "s#path = \"[^\"]*\"#path = \"$F\"#" -e "s#name = \"repro-$id\"#name = \"ctl-$id\"#" /work/repro/$id/Cargo.toml > $C/Cargo.toml
cp /work/fix/$sha/Cargo.lock $C/Cargo.lock
cd $C || exit 2
shift 3
case $mode in
  native) cargo run --offline -q -- "$@" 2>&1 | grep -v '^warning' | tail -30 ;;
  native-release) cargo run --offline -q --release -- "$@" 2>&1 | grep -v '^warning' | tail -30 ;;
  miri-x86) MIRI_SYSROOT=/work/miri-sysroot-x86 cargo +nightly-2026-10-01 miri run --offline -q --target x86_64-unknown-linux-gnu -- "$@" 2>&1 | grep -v '^warning' | tail -25 ;;
  miri-x86-release) MIRI_SYSROOT=/work/miri-sysroot-x86 cargo +nightly-2026-10-01 miri run --offline -q --release --target x86_64-unknown-linux-gnu -- "$@" 2>&1 | grep -v '^warning' | tail -25 ;;
  miri-aarch64) MIRIFLAGS=-Zmiri-symbolic-alignment-check cargo +nightly-2026-10-01 miri run --offline -q -- "$@" 2>&1 | grep -v '^warning' | tail -25 ;;
esac
echo "(control exit of pipeline: $?)"
