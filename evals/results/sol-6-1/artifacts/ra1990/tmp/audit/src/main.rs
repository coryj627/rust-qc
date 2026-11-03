use pulldown_cmark::{Parser,Options,Event,html};
use std::{fs,env};
fn check(s:&str,o:Options) {
 let mut stack=Vec::new();
 let events:Vec<_>=Parser::new_ext(s,o).into_offset_iter().map(|(e,r)|{
 assert!(r.start<=r.end && r.end<=s.len() && s.is_char_boundary(r.start) && s.is_char_boundary(r.end),"bad range {r:?} {e:?}");
 match &e { Event::Start(t)=>stack.push(t.to_end()),Event::End(t)=>assert_eq!(stack.pop(),Some(*t)),_=>()}; e }).collect();
 assert!(stack.is_empty()); let mut out=String::new();html::push_html(&mut out,events.into_iter());
}
fn rnd(x:&mut u64,n:usize)->usize{*x^=*x<<13;*x^=*x>>7;*x^=*x<<17; (*x as usize)%n}
fn main(){ let args:Vec<_>=env::args().collect(); match args.get(1).map(|x|x.as_str()) {
 Some("render")=>{let s=fs::read_to_string(&args[2]).unwrap();let o=Options::from_bits_truncate(args.get(3).map(|v|v.parse().unwrap()).unwrap_or(0));for x in Parser::new_ext(&s,o).into_offset_iter(){println!("{x:?}");}let mut h=String::new();html::push_html(&mut h,Parser::new_ext(&s,o));println!("{h}");},
 Some("labels")=>{for (c,n) in [("a",999),("a",1000),("a",10000),("é",499),("é",500),("😀",249),("😀",250)] {let l=c.repeat(n);let s=format!("[x][{l}]\n\n[{l}]: /url\n");let p=Parser::new(&s);let defs=p.reference_definitions().iter().count();let links=p.filter(|e|matches!(e,Event::Start(pulldown_cmark::Tag::Link{..}))).count();println!("{c:?} x {n}: defs={defs}, links={links}");}},
 Some("fuzz")=>{std::panic::set_hook(Box::new(|_|{}));let mut x=123456789;let mut corpus=vec!["".to_string()];if let Ok(paths)=fs::read_dir("/work/crate/pulldown-cmark/tests/suite"){for p in paths.flatten(){if let Ok(s)=fs::read_to_string(p.path()){for part in s.split("```````````````````````````````` example").skip(1){if let Some((md,_))=part.split_once("\n.\n"){corpus.push(md.trim_start_matches('\n').to_string());}}}}}
 let tokens=["a","b"," ","\n","\r","\t","*","_","~","^","$","{","}","[","]","!",":","-","+",">","<","`","\\","|","\"","'","(",")","#","é","😀","\0","&",";","=",".","[^a]: ","\n: ","\n---\n","\n|--|\n","[[a|","```\n", "[x](a)"];
 let n=args.get(2).map(|s|s.parse().unwrap()).unwrap_or(1000000);
 for i in 0..n { let ci=rnd(&mut x,corpus.len());let mut s=corpus[ci].clone();for _ in 0..1+rnd(&mut x,12){let op=rnd(&mut x,5);let mut chars:Vec<_>=s.chars().collect();if op==0 && !chars.is_empty(){let a=rnd(&mut x,chars.len());let b=a+rnd(&mut x,chars.len()-a+1);chars.drain(a..b);s=chars.into_iter().collect();}else{let a=rnd(&mut x,chars.len()+1);let b=s.char_indices().nth(a).map(|(i,_)|i).unwrap_or(s.len());let t=tokens[rnd(&mut x,tokens.len())];s.insert_str(b,t);}}
 let bits=match rnd(&mut x,4){0=>0,1=>Options::all().bits(),2=>Options::all().bits() & !512,_=>rnd(&mut x,1<<16) as u32};let o=Options::from_bits_truncate(bits);
 if std::panic::catch_unwind(||check(&s,o)).is_err(){fs::write("/work/tmp/failure.md",&s).unwrap();eprintln!("FAIL iter={i} bits={bits} input={s:?}");std::process::exit(1);}
 if s.len()<1500 && corpus.len()<10000 && rnd(&mut x,10)==0 {corpus.push(s);} if i%100000==0{eprintln!("iter={i} corpus={}",corpus.len());}}
 },_=>panic!("mode")}}
