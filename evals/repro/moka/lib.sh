# Shared helper: copies one reproducer to /tmp and points its path dependency at MOKA_DIR.
# MOKA_DIR defaults to the baseline tree; set it to /work/fix/<sha> for the fix control.
prepare() {
    id=$1
    HERE=/work/repro/$id
    MOKA_DIR=${MOKA_DIR:-/work/base}
    BUILD=/tmp/build-$id
    mkdir -p "$BUILD"
    sed "s#@MOKA@#$MOKA_DIR#" "$HERE/Cargo.toml.in" > "$BUILD/Cargo.toml"
    cp -r "$HERE/src" "$BUILD/"
    if [ "$MOKA_DIR" = /work/base ] && [ -f "$HERE/Cargo.lock" ]; then
        cp "$HERE/Cargo.lock" "$BUILD/Cargo.lock"
    fi
    cd "$BUILD" || exit 2
    echo "== moka tree: $MOKA_DIR =="
}
