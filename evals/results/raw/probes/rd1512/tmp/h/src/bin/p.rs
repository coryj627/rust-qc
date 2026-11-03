use pulldown_cmark::*;
fn show(s:&str,o:Options,off:bool){
 let s2=s.to_string();
 let r=std::panic::catch_unwind(move||{ let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&s2,o)); h});
 println!("--- {:?} opts={:#x}",s,o.bits());
 match r { Ok(h)=>println!("{:?}",h), Err(e)=>{println!("PANIC {:?}",e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|x|x.to_string())));return;} }
 if off { for (e,r) in Parser::new_ext(s,o).into_offset_iter(){ println!("   {:?} {:?}={:?}",e,r,s.get(r.clone())); } }
}
fn main(){
 let e=Options::empty(); let a=Options::all();
 let gfm=Options::ENABLE_TABLES|Options::ENABLE_STRIKETHROUGH|Options::ENABLE_TASKLISTS|Options::ENABLE_FOOTNOTES|Options::ENABLE_GFM;
 let st=Options::ENABLE_STRIKETHROUGH; let sub=Options::ENABLE_SUBSCRIPT; let sup=Options::ENABLE_SUPERSCRIPT;let wiki=Options::ENABLE_WIKILINKS;let t=Options::ENABLE_TABLES;let hattr=Options::ENABLE_HEADING_ATTRIBUTES;let math=Options::ENABLE_MATH;

 let meta=Options::ENABLE_YAML_STYLE_METADATA_BLOCKS; let fnn=Options::ENABLE_FOOTNOTES; let gfmq=Options::ENABLE_GFM;
 show("[a]: /u\n    \nb",e,true); show("- [a]: /u\n      \n- b",e,false); show("[a]: /u\n    ",e,true); show("[a]: /u\n     \n",e,false); show("> - [a]: /u\n>       \n",e,false);
 show("```\na\n```\t\nb",e,false); show("```\na\n``` \nb",e,false);
 show("```\rfoo\r```\rbar",e,false); show("    a\r    b\r\rc",e,false); show("<div>\r\rfoo",e,false); show("# a\rb\r===\rc",e,false); show("<!-- a -->\rb",e,false);show("```\nfoo\rx",e,false);
 let l=format!("[{}]\n\n[{}]: /u","a".repeat(1000),"a".repeat(1000)); show(&l[..20],e,false); { let h=std::panic::catch_unwind(||0); let _=h; let mut o=String::new(); html::push_html(&mut o,Parser::new(&l)); println!("ascii1000 link? {}",o.contains("<a href")); }
 let l=format!("[{}]\n\n[{}]: /u","漢".repeat(400),"漢".repeat(400)); { let mut o=String::new(); html::push_html(&mut o,Parser::new(&l)); println!("cjk400 link? {}",o.contains("<a href")); }
 let l=format!("[{}]\n\n[{}]: /u","a".repeat(999),"a".repeat(999)); { let mut o=String::new(); html::push_html(&mut o,Parser::new(&l)); println!("ascii999 link? {}",o.contains("<a href")); }
 show("x <![CDATA[a]b]]> y",e,false); show("x <![CDATA[a]> y",e,false); show("x <![CDATA[a]]> y",e,false);show("x <![CDATA[a]]]> y",e,false);
 show("x[^A]\n\n[^a]: y\n",fnn,false);
 show("| h |\n|---|\n| [[a\\|b]] |\n",t|wiki,true); show("[[a|*b*]]",wiki,false); show("[[`a|b`]]",wiki,false);
 show("a|b\n-:-|-\n",t,false); show("a|b\n- -|-\n",t,false); show("a|b\n-\t|\t-\n",t,false);
 show("<a b=c\td=e> x",e,false); show("x <a b=c\td=e>",e,false);
 show("~~[x](/y)~~ ~~`c`~~",sub,false);
 show("# *a* #",e,true);
 show("foo\n```\tinfo\nx\n```",e,false);
 show("![a[^1]](/u)\n\n[^1]: x",gfm,false);
 show("> [!NOTE]\n> a",gfmq,false);
 show("![x](y \"t\")\n[a](b 'c\\'d')",e,false);
 show("[a](<b)c> \"t\")",e,false);
 show("*a\\\n*",e,false);
 show("a\\\n",e,false);
 show("\\``a`",e,true);
 show("a<\nb>",e,false); show("a< b>",e,false); show("<  \nb>",e,false);show("<\\\nb>",e,false);
 show("[a]: /u 'b\n\nc'\n\n[a]",e,false);
 show("# a\n{#x}\n===",hattr,false);
 show("a {#x}\n===",hattr,false);show("a\\{#x}\n===",hattr,false);show("# a\\{#x}",hattr,false);show("# {#x}",hattr,true);show("# \\{#x}",hattr,true);show("a  {#x}\n===",hattr,true);show("a\nb  {#x}\n===",hattr,true);
 show("$$a$$ $b$ $$c$ $d$$",math,false); show("$a$$b$",math,false); show("$$\na\n$$",math,false); show("> $a\n> b$",math,false);show("| $a\\|b$ |\n|-|",t|math,false);

}
