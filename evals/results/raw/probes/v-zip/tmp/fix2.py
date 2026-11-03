p='probe/src/main.rs'; s=open(p).read()
s=s.replace('let hs=a.by_index_raw(i).unwrap().header_start() as usize; println!("local cs','let hs={let f=a.by_index_raw(i).unwrap(); f.header_start() as usize}; println!("local cs')
s=s.replace('println!("ro {:o} rw {:o}",a.by_index_raw(0).unwrap().unix_mode().unwrap(),a.by_index_raw(1).unwrap().unix_mode().unwrap());','{let m0=a.by_index_raw(0).unwrap().unix_mode().unwrap(); let m1=a.by_index_raw(1).unwrap().unix_mode().unwrap(); println!("ro {:o} rw {:o}",m0,m1);}')
open(p,'w').write(s)
