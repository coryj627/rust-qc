use pulldown_cmark::*; use std::time::Instant;
fn show(s:&str,o:Options,off:bool){
 let s2=s.to_string();
 let r=std::panic::catch_unwind(move||{ let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&s2,o)); h});
 println!("--- {:?} opts={:#x}",s,o.bits());
 match r { Ok(h)=>println!("{:?}",h), Err(e)=>{println!("PANIC {:?}",e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|x|x.to_string())));return;} }
 if off { for (e,r) in Parser::new_ext(s,o).into_offset_iter(){ println!("   {:?} {:?}={:?}",e,r,s.get(r.clone())); } }
}
fn tm(name:&str,o:Options,f:&dyn Fn(usize)->String,base:usize){ let mut n=base; for _ in 0..5 { let s=f(n); let st=Instant::now(); let c=Parser::new_ext(&s,o).count(); let d=st.elapsed().as_secs_f64(); println!("{} n={} bytes={} ev={} t={:.3}",name,n,s.len(),c,d); if d>6.0 {break;} n*=2; } }
fn main(){
 let e=Options::empty(); let t=Options::ENABLE_TABLES;
 show("> foo <!-- a\n> b -->",e,false); show("> <?x\n> y?>",e,false); show("> a <![CDATA[b\n> c]]>",e,false); show("- a <!x\n  c>",e,false);show("- a <!-- b\n  c -->",e,false);
 show("/\n:é\n\t+ [",Options::ENABLE_DEFINITION_LIST,true);
 show(">\t+ a",e,true); show("- a\n\t+ b",e,true);
 show("| a |\n|---|\n| <a b=\"c\\|d\"> |\n| <http://a\\|b> |\n| [x](<b\\|c>) |",t,false);
 let hdr="|a".repeat(1000)+"|\n"+&"|-".repeat(1000)+"|\n"; let doc=hdr+&"x\n".repeat(300)+"\n# after\n\nparagraph\n";
 let ev:Vec<_>=Parser::new_ext(&doc,t).collect(); let mut depth=0; let mut hd=None; for (i,x) in ev.iter().enumerate(){ match x { Event::Start(Tag::Table(_))=>depth+=1, Event::End(TagEnd::Table)=>depth-=1, Event::Start(Tag::Heading{..})=>{hd=Some((i,depth));} _=>{} } }
 let rows=ev.iter().filter(|x|matches!(x,Event::Start(Tag::TableRow))).count();
 println!("table-limit: rows={} heading (idx,table_depth)={:?} last events {:?}",rows,hd,&ev[ev.len().saturating_sub(8)..]);
 let doc2="|a".repeat(1000)+"|\n"+&"|-".repeat(1000)+"|\n"+&"x\n".repeat(200)+"\n# after\n";
 let ev:Vec<_>=Parser::new_ext(&doc2,t).collect(); println!("control(200 rows) last events {:?}",&ev[ev.len()-5..]);
 tm("*a_",e,&|n|"*a_ ".repeat(n),40000); tm("_a*",e,&|n|"_a* ".repeat(n),40000);
 tm("yaml",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,&|n|"---\t\na\n".repeat(n),5000);
 let f=std::fs::read_to_string("/work/tmp/ents.txt").unwrap(); let mut bad=0; let mut tot=0; for l in f.lines(){ let (n,v)=l.split_once(' ').unwrap(); let v:String=v.split(',').map(|x|char::from_u32(x.parse().unwrap()).unwrap()).collect(); let src=format!("x&{};",n); let got:String=Parser::new(&src).filter_map(|e|if let Event::Text(t)=e{Some(t.to_string())}else{None}).collect(); tot+=1; if got!=format!("x{}",v){ bad+=1; if bad<20 {println!("ENT {} want {:?} got {:?}",n,v,got);} } } println!("entities tot={} bad={}",tot,bad);
}
