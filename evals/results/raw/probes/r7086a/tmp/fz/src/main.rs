use std::{fs, panic, sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}, collections::BTreeMap};
const TOK: &[&str] = &["a","b","foo",":",":=","=","\"","'","`","'''","\"\"\"","```","{{","}}","{{{{","\n","\n\n"," ","  ","\t","#","#!","\\","\\\n","(",")","[","]","@","-","?","$","*","+","/","&&","||","==","!=","=~","!~",",","::","if ","else ","assert(","f\"","f'","f'''","x\"","x'","set ","export ","unexport ","import ","mod ","alias ","true","false","shell","[private]","[arg('a',long)]","[doc('x')]","[group('g')]","[script]","[confirm]","\r\n","é","日本","\u{feff}","\\u{","\\n","\u{0}","\u{7f}","\u{2028}", "set guards\n","set ignore-comments\n", "eager ", "default"];
const SEEDS: &[&str] = &[
"foo:\n  echo hi\n",
"x := 'a'\ny := \"b\\n\"\nz := `echo`\nfoo a b='c' +d: x (y a) && z\n  @echo {{a}}\n  -echo {{{{\n\n  echo \\\n    b\n",
"set shell := ['a', 'b']\nset dotenv-load\nexport a := f'x{{b}}y{{c}}z'\nb:='''\n  q\n   r\n'''\nc:=if a == b { c } else if d =~ 'e' { f } else { g }\n",
"[private]\n[doc('x')]\n[group('g')]\nfoo:\n #!/bin/sh\n  echo\n\n  x\n# c\nalias a := foo\nmod? b\nimport? 'c'\n",
"[arg('a', long='x', short='y', pattern='p', value='v', help='h')]\nfoo a='':\n  echo\nf(a, b) := a + b\nunexport Q\nx := assert(a == b, 'm')\ny := / 'a' / 'b' && c || d\n",
"# doc\nfoo: # comment\n  echo {{ f\"{{ a }}\" }}\n\tb\n",
];
struct R(u64);
impl R { fn n(&mut self)->u64{ self.0^=self.0<<13; self.0^=self.0>>7; self.0^=self.0<<17; self.0 } fn u(&mut self,m:usize)->usize{(self.n()%(m as u64)) as usize} }
fn mutate(r:&mut R)->String{
  let mut s:String=SEEDS[r.u(SEEDS.len())].to_string();
  if r.u(5)==0 { s.clear(); for _ in 0..(1+r.u(12)) { s.push_str(TOK[r.u(TOK.len())]); } return s; }
  for _ in 0..(1+r.u(6)) {
    let bs: Vec<usize> = s.char_indices().map(|(i,_)|i).chain(std::iter::once(s.len())).collect();
    let i=bs[r.u(bs.len())]; let j=bs[r.u(bs.len())];
    match r.u(4) { 0=> s.insert_str(i,TOK[r.u(TOK.len())]), 1=>{ let (a,b)=(i.min(j),i.max(j)); let b=b.min(a+8).min(s.len()); let b= (b..=s.len()).find(|x|s.is_char_boundary(*x)).unwrap(); s.replace_range(a..b,""); }, 2=>{ let (a,b)=(i.min(j),i.max(j)); let c=s[a..b].to_string(); if c.len()<40 { s.insert_str(a,&c);} }, _=>{ s.truncate(i); } }
  }
  s
}
fn main(){
  let secs:u64=std::env::args().nth(1).unwrap().parse().unwrap();
  panic::set_hook(Box::new(|_|{}));
  let found=Arc::new(Mutex::new(BTreeMap::<String,(String,String)>::new()));
  let count=Arc::new(AtomicU64::new(0));
  let start=std::time::Instant::now();
  let mut hs=vec![];
  for t in 0..4u64 {
    let (found,count)=(found.clone(),count.clone());
    hs.push(std::thread::spawn(move||{
      let dir=format!("/tmp/fz{t}"); fs::create_dir_all(&dir).unwrap();
      let path=format!("{dir}/justfile");
      let mut r=R(0x9E3779B97F4A7C15u64.wrapping_mul(t+1));
      let cmds:&[&[&str]]=&[&["--list"],&["--dump"],&["--dump","--dump-format","json"],&["--summary"],&["--fmt","--check"],&["--variables"],&["--groups"],&["--show","foo"],&["--usage","foo"]];
      while start.elapsed().as_secs()<secs {
        let s=mutate(&mut r);
        fs::write(&path,&s).unwrap();
        let c=cmds[r.u(cmds.len())];
        let mut a:Vec<String>=vec!["just".into(),"--unstable".into(),"--justfile".into(),path.clone(),"--working-directory".into(),dir.clone()];
        a.extend(c.iter().map(|x|x.to_string()));
        let res=panic::catch_unwind(||{ let _=just::run(a.iter().map(|x|x.as_str())); });
        count.fetch_add(1,Ordering::Relaxed);
        if let Err(e)=res {
          let m=e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|x|x.to_string())).unwrap_or("?".into());
          let k:String=m.chars().take(70).collect();
          let mut f=found.lock().unwrap();
          let en=f.entry(k).or_insert((s.clone(),c.join(" ")));
          if s.len()<en.0.len(){ *en=(s.clone(),c.join(" ")); }
        }
      }
    }));
  }
  for h in hs { let _=h.join(); }
  use std::io::Write;
  let mut o=fs::File::create("/work/tmp/fz.out").unwrap();
  writeln!(o,"runs {}",count.load(Ordering::Relaxed)).unwrap();
  for (k,(s,c)) in found.lock().unwrap().iter(){ writeln!(o,"PANIC {k}\n  cmd {c}\n  input {s:?}").unwrap(); }
}
