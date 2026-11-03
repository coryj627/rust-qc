p='probe/src/main.rs'; s=open(p).read()
s=s.replace('let mut a=open(&o).unwrap(); let r=a.by_name_decrypt("b",b"pw"); match r {Ok(mut f)=>{println!("enc={}",f.encrypted());let mut v=vec![];println!("{:?}",f.read_to_end(&mut v).map_err(|e|e.to_string()))},Err(e)=>println!("decrypt err {e:?}")}; println!("enc flag {}", a.by_index_raw(0).unwrap().encrypted());',
 'let mut a=open(&o).unwrap(); {let r=a.by_name_decrypt("b",b"pw"); match r {Ok(mut f)=>{println!("enc={}",f.encrypted());let mut v=vec![];println!("{:?}",f.read_to_end(&mut v).map_err(|e|e.to_string()))},Err(e)=>println!("decrypt err {e:?}")};} let en=a.by_index_raw(0).unwrap().encrypted(); println!("enc flag {}", en);')
open(p,'w').write(s)
