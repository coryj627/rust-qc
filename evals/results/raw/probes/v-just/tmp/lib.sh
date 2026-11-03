J=/target/debug/just
[ -x $J ] || (cd /work/crate && cargo build --locked --bin just 2>/dev/null)
export JUST_UNSTABLE=
unset JUST_UNSTABLE
ROOT=/work/tmp/w
rm -rf $ROOT; mkdir -p $ROOT
n=0
# t name: new dir
t() { n=$((n+1)); D=$ROOT/$n; mkdir -p $D; cd $D; echo; echo "=== [$n] $1"; }
r() { echo "\$ just $*"; timeout 20 $J "$@" </dev/null 2>&1 | head -${HL:-25}; echo "[exit ${PIPESTATUS:-?}]"; }
rr() { echo "\$ just $*"; timeout 20 $J "$@" </dev/null > .out 2>&1; c=$?; head -${HL:-25} .out; echo "[exit $c]"; }
