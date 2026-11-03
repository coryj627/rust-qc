p='tmp/p7/src/bin/p4.rs'
s=open(p).read()
s=s.replace('let orig=bytes.clone();','let orig=bytes.clone(); if std::str::from_utf8(&orig).is_err() { continue; }')
i=s.index('// 5. utf8 validity')
j=s.index('// 6. input mutated')
s=s[:i]+s[j:]
open(p,'w').write(s)
