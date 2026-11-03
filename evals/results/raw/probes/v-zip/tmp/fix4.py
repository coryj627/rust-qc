p='probe/runall.sh'; s=open(p).read()
s=s.replace('''timeout 120 $B $t 2>&1 | cut -c1-400 | grep -v "RUST_BACKTRACE"
  echo "--- exit $?"''','''timeout 120 $B $t > /work/tmp/one.out 2>&1
  rc=$?
  cut -c1-400 /work/tmp/one.out | grep -v "RUST_BACKTRACE"
  echo "--- exit $rc"''')
open(p,'w').write(s)
