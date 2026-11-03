use pulldown_cmark::*;
fn h(s:&str,o:Options)->String{let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(s,o)); h}
fn short(s:&str)->String{ if s.len()>260 {format!("{}...{}",&s[..s.char_indices().nth(160).unwrap().0],&s[s.char_indices().rev().nth(90).unwrap().0..])} else {s.to_string()} }
fn main(){
 { let head=format!("{}|\n{}|\n","|a".repeat(512),"|-".repeat(512)); let t=format!("{head}{}\n","|\n".repeat(0)+&"|x|\n".repeat(514)); let doc=t.repeat(60); let st=std::time::Instant::now(); let n=Parser::new_ext(&doc,Options::ENABLE_TABLES).count(); let d=st.elapsed().as_secs_f64(); let st=std::time::Instant::now(); let o=h(&doc,Options::ENABLE_TABLES); println!("table-amp doc={} events={} {:.2}s html={} {:.2}s",doc.len(),n,d,o.len(),st.elapsed().as_secs_f64()); }
 println!("{:?}",h("a\n\x0c\nb\n",Options::empty())); println!("{:?}",h("#\x0cfoo\n",Options::empty())); println!("{:?}",h("-\x0cfoo\n",Options::empty()));
 if std::env::args().nth(1).is_some(){return;}

 // label length
 for n in [999,1000,1001,5000]{ let l="a".repeat(n); let s=format!("[{l}]\n\n[{l}]: /u\n"); println!("ascii label {n}: link={}",h(&s,Options::empty()).contains("<a href")); }
 for n in [300,333,334,500,999]{ let l="日".repeat(n); let s=format!("[{l}]\n\n[{l}]: /u\n"); println!("cjk label {n} chars: link={}",h(&s,Options::empty()).contains("<a href")); }
 // table cell cap
 let cols=1000; let head=format!("{}|\n{}|\n","|a".repeat(cols),"|-".repeat(cols));
 let body="|x|\n".repeat(270); let s=format!("{head}{body}\nafter *para*\n\n> quote\n");
 let out=h(&s,Options::ENABLE_TABLES); println!("table cap: {}",short(&out)); println!("table count rows={} ",out.matches("<tr>").count());
 let s2=format!("> {}|\n> {}|\n{}\nafter\n","|a".repeat(cols),"|-".repeat(cols),"> |x|\n".repeat(270));
 let r=std::panic::catch_unwind(||{ let v:Vec<_>=Parser::new_ext(&s2,Options::ENABLE_TABLES).into_offset_iter().collect(); v.len()}); println!("table cap in quote offset iter: {:?}",r.is_ok());
 let r=std::panic::catch_unwind(||{ let s3=format!("{head}{}\nafter\n","   |x|\n".repeat(270)); let c=Parser::new_ext(&s3,Options::ENABLE_TABLES).into_offset_iter().filter(|(_,r)|r.start>r.end).count(); c}); println!("table cap indented: {:?}",r);
 // expansion limit
 let uri="x".repeat(40_000); let s=format!("[logo]: data:{uri}\n\n[logo] [logo] [logo] [logo]\n"); let out=h(&s,Options::empty()); println!("expansion: doc={} links={} literal={}",s.len(),out.matches("<a href").count(),out.matches("[logo]").count());
 // closing tag
 println!("{:?}",h("a </b\n    > c\n",Options::empty()));
 println!("{:?}",h("a <b\n    > c\n",Options::empty()));
 // broken link callback
 let mut calls=vec![]; let cb=|l:BrokenLink<'_>|{ calls.push((l.reference.to_string(),l.span.clone(),l.link_type)); None };
 let t="[a][b] [c][] [d] ![e] [f][g][h]\n"; let _:Vec<_>=Parser::new_with_broken_link_callback(t,Options::empty(),Some(cb)).collect(); println!("broken calls {calls:?}");
 // Text merge
 let v:Vec<_>=TextMergeStream::new(vec![Event::Text("".into()),Event::Rule].into_iter()).collect(); println!("merge {v:?}");
 // labels
 for (s,o) in [("x[^a]\n\n[^a]: y\n[^a]: z\n",Options::ENABLE_FOOTNOTES),("[x]: /a\n[X]: /b\n\n[x]\n",Options::empty()),("[[a`|`b]]\n",Options::ENABLE_WIKILINKS),("[[a\nb|c]]\n",Options::ENABLE_WIKILINKS),("> <?a\n> b?> <![CDATA[c\n> d]]> <!X e\n> f>\n",Options::empty()),("- x <?a\n  b?>\n",Options::empty()),("a\n<search>\n",Options::empty()),("> a\n>\n> : b\n",Options::ENABLE_DEFINITION_LIST),("> [^a]:\n> b\n",Options::ENABLE_OLD_FOOTNOTES)]{ println!("{s:?} => {:?}",h(s,o)); }
}
