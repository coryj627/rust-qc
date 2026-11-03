p='probes/meta/src/main.rs'
s=open(p).read()
s=s.replace(' inmd=inmd&&false; continue; }',' continue; }')
open(p,'w').write(s)
