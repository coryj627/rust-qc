export RUST_BACKTRACE=0; cd /work/tmp/probe; export CARGO_TARGET_DIR=/target/probe2
cargo build --offline --release --bin p3 2>&1 | tail -1
timeout 100 /target/probe2/release/p3 dump 2>&1 | cut -c1-140
python3 - <<'PY'
import zipfile,glob,os
for f in sorted(glob.glob('/work/tmp/samples/*.zip')):
    try:
        z=zipfile.ZipFile(f); n=z.namelist(); t=z.testzip(); print('python',os.path.basename(f),len(n),n[:3],'testzip',t)
    except Exception as e: print('python',os.path.basename(f),'ERR',e)
PY
cd /work/crate; export CARGO_TARGET_DIR=/target/miri
timeout 500 cargo +nightly-2026-10-01 miri test --offline --locked --lib --no-default-features --features aes-crypto -- spec:: zipcrypto aes 2>&1 | tail -4 | cut -c1-200
