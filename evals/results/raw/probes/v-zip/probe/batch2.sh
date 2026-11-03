R=/target/release/probe; D=/target/debug/probe
for bin in $R $D; do for t in "shrink_stack 20000" "shrink_stack 100000" "shrink_stack 200000" "lzma_skip" "ppmd_skip" "ioerr2" "append_comment" "merge_big"; do echo "=== $bin $t"; timeout 120 $bin $t > /work/tmp/one.out 2>&1; rc=$?; grep -v -E '^ +[0-9]+: |^ +at |RUST_BACKTRACE|^$' /work/tmp/one.out | cut -c1-200 | head -14; echo "--- exit $rc"; done; done
echo == seekcount; timeout 120 $R seekcount 2>&1 | tail -3
echo == flakyloop debug; timeout 200 $D flakyloop > /work/tmp/one.out 2>&1; echo rc=$?; grep -E "panicked|assert|destructor|^done|assertion" /work/tmp/one.out | sort | uniq -c | head
echo == flakyloop rel; timeout 200 $R flakyloop > /work/tmp/one.out 2>&1; echo rc=$?; grep -E "panicked|^done" /work/tmp/one.out | sort | uniq -c | head
echo == stream_large; timeout 300 $R stream_large 2>&1 | cut -c1-200
for k in 500 1000 2000 4000 8000; do timeout 200 $R quad $k 2>&1 | cut -c1-200; done
for n in 40000 80000 160000; do timeout 100 $R overlap $n 2>&1 | cut -c1-200; done
echo == huff debug; timeout 150 $D huff 12345 300000 > /work/tmp/one.out 2>&1; echo rc=$?; grep -E "panicked|^done" -A1 /work/tmp/one.out | head -6
echo == zopfli; cd /work/probe2; cargo build --offline 2>&1 | grep -E "^error|Finished"; for l in 1 5 6 9 10 11 24 0; do echo "-- $l"; timeout 60 /target/debug/probe2 $l 2>&1 | grep -v -E '^ +[0-9]+: |^ +at |RUST_BACKTRACE|^$' | cut -c1-160 | head -6; done
cd /work/tmp
echo == interop
cat > /work/tmp/py.py <<'PY'
import zipfile,sys,glob
for f in sorted(glob.glob('/work/tmp/x_*.zip')+glob.glob('/work/tmp/cnt_*.zip')+['/work/tmp/trunc.zip','/work/tmp/fake_eocd.zip','/work/tmp/st_nodesc.zip','/work/tmp/st_crypto.zip','/work/tmp/aes_after.zip','/work/tmp/aes_before.zip','/work/tmp/ua_chinese.zip']):
    try:
        z=zipfile.ZipFile(f); n=len(z.namelist()) if len(z.namelist())>5 else z.namelist(); print(f.split('/')[-1],'py ok',n,end=' ')
        try:
            print(z.testzip() if 'crypto' not in f and 'aes' not in f else z.read(z.namelist()[0],pwd=b'pw'))
        except Exception as e: print('read err',repr(e)[:80])
    except Exception as e: print(f.split('/')[-1],'py err',repr(e)[:90])
try:
    import lzma; print('py lzma', lzma.decompress(open('/work/tmp/empty.xz','rb').read()))
except Exception as e: print('py lzma err',repr(e))
PY
python3 /work/tmp/py.py
for f in st_crypto st_nodesc trunc fake_eocd x_stale_path_crc x_comment_crc_cp437 cnt_0_2 cnt_1_2 x_ntfs_len36 x_ut_len9_flags1; do echo "-- unzip $f"; unzip -P pw -t $f.zip 2>&1 </dev/null | tail -4 | cut -c1-140; done
