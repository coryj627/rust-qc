import re
p='probe/src/main.rs'; s=open(p).read()
a=s.index('let mut z=ZipWriter::new(Cursor::new(vec![])); {let mut a=open(&b).unwrap(); z.raw_copy_file(a.by_name(&n[0])')
b=s.index('let o=z.finish().unwrap().into_inner(); let mut a=open(&o).unwrap(); println!("after raw {}"')
s=s[:a]+'let mut z=ZipWriter::new(Cursor::new(vec![])); {let mut a=open(&b).unwrap(); z.raw_copy_file(a.by_index_raw(0).unwrap()).unwrap();} '+s[b:]
s=s.replace('let p=aa.by_name_seek("p").map(|_|()).is_ok(); (aa.by_index_raw(0).unwrap().data_start(),aa.by_index_raw(1).unwrap().data_start()) }','let x=aa.by_index_raw(0).unwrap().data_start(); let y=aa.by_index_raw(1).unwrap().data_start(); (x,y) }')
s=s.replace('let hs=a.by_name("r").map(|f|f.header_start()).unwrap() as usize;','let hs={let f=a.by_name("r").unwrap(); f.header_start() as usize};')
open(p,'w').write(s)
