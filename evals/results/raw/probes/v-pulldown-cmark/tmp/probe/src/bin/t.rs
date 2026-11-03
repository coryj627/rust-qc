use probe::*; use pulldown_cmark::*; use std::time::Instant;
fn rep(s:&str,n:usize)->String{s.repeat(n)}
fn gen(name:&str,n:usize)->(Options,String){
 let e=Options::empty();
 match name{
  "title"=>(e,format!("[a](b \"{}",rep("x\n",n))),
  "emph_a"=>(e,rep("*a_ ",n)),
  "emph_ctl"=>(e,rep("*a* ",n)),
  "emph_b"=>(e,format!("{}{}",rep("*a ",n),rep("b_ ",n))),
  "deep"=>(e,format!("{}a\n{}",rep("- ",n),rep("\n",n))),
  "yaml_tab"=>(opts("yaml"),rep("---\t\na\n\n",n)),
  "yaml_q"=>(opts("yaml"),rep("> ---\n",n)),
  "yaml_q2"=>(opts("yaml"),rep("> ---\n> a\n>\n",n)),
  "wikiimg"=>(opts("wiki"),format!("{}{}",rep("![[a ",n),rep("]]",n))),
  "fn1"=>(opts("none"),rep("[^a]: b\n",n)),
  "fn1f"=>(opts("fn"),rep("[^a]: b\n",n)),
  "fn2"=>(e,rep("[^a x\n",n)),
  "crfence"=>(e,rep("```x\r",n)),
  "wikiempty"=>(opts("wiki"),rep("[[a|]] ",n)),
  _=>panic!()
 }
}
fn main(){
 let a:Vec<String>=std::env::args().collect();
 let name=&a[1]; let mut n:usize=a[2].parse().unwrap(); let steps:usize=a[3].parse().unwrap();
 let limit=a.get(4).map(|x|x.parse::<f64>().unwrap()).unwrap_or(30.0);
 for _ in 0..steps{
  let (o,s)=gen(name,n);
  let t=Instant::now();
  let mut c=0u64; for _ev in Parser::new_ext(&s,o){c+=1; if c>200_000_000 {println!("event cap hit"); break;}}
  let d=t.elapsed().as_secs_f64();
  println!("{name} n={n} bytes={} events={c} {:.3}s",s.len(),d);
  if d>limit {break}
  n*=2;
 }
}
