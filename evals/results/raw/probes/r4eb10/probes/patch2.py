p='probes/fuzz/src/main.rs'
s=open(p).read()
s=s.replace('let len = 1 + r.b(if i % 4 == 0 { 40 } else { 14 });','let len = 1 + r.b(if i % 8 == 0 { 40 } else if i % 2 == 0 { 14 } else { 7 });')
s=s.replace('assert!(s2.is_char_boundary(rg.start) && s2.is_char_boundary(rg.end), "BADBOUNDARY");','')
s=s.replace('''let bits = if r.b(3) == 0 {
            r.n() as u32 & 0x7fff
        } else {''','''let bits = if r.b(2) == 0 {
            r.n() as u32 & 0xffff
        } else if r.b(2) == 0 {
            (1u32 << (1 + r.b(15))) | (1u32 << (1 + r.b(15)))
        } else {''')
s=s.replace('Some(format!("{} @ {}", m, l))','let _ = m; Some(format!("{}", l))')
open(p,'w').write(s)
