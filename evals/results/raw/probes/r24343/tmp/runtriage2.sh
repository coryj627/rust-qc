export RUST_BACKTRACE=0; cd /work/tmp; export RUST_BACKTRACE=0
big=""; four=""; for f in $(ls abort-*.bin); do
  o=$( ( ulimit -v 1500000; timeout 30 /target/probe2/release/triage $f 2>&1 ) | tail -3 | cut -c1-120)
  case "$o" in *39191576575984*) big="$big $f";; *"allocation of 4294967108"*) four="$four $f";; esac
  [ -n "$big" ] && [ -n "$four" ] && break
done
echo big=$big four=$four
for f in $(echo $big | cut -d' ' -f1) $(echo $four | cut -d' ' -f1); do echo "== $f"; ls -la $f; ( ulimit -v 1500000; /target/probe2/release/triage $f 2>&1 | tail -6 | cut -c1-200 )
python3 - "$f" <<'PY'
import sys,struct
b=open(sys.argv[1],'rb').read(); print(len(b))
i=b.rfind(b'PK\x05\x06'); print('eocd@',i, struct.unpack('<HHHHIIH',b[i+4:i+22]))
j=b.rfind(b'PK\x06\x06'); print('eocd64@',j, struct.unpack('<QHHIIQQQQ',b[j+4:j+56]) if j>=0 else None)
k=b.rfind(b'PK\x06\x07'); print('loc@',k, struct.unpack('<IQI',b[k+4:k+20]) if k>=0 else None)
p=0
while True:
    p=b.find(b'PK\x03\x04',p)
    if p<0: break
    print(' LFH@',p,'method',struct.unpack('<H',b[p+8:p+10])[0],'cs,us',struct.unpack('<II',b[p+18:p+26]),'name',b[p+30:p+30+struct.unpack('<H',b[p+26:p+28])[0]][:30]); p+=4
    if p>2000: break
PY
done
