cd /work/tmp/p13; J=/work/tmp/bin/just; ulimit -c 0
python3 fnfuzz.py 2>&1 | cut -c1-260
$J -f mfail top; echo rc=$?
mkdir -p gone; cd gone; rmdir ../gone; $J --version; $J -f /work/tmp/p13/env5; echo rc=$?; cd /work/tmp/p13
