use pulldown_cmark::*; use std::panic;
fn fails(s:&str,o:Options)->bool{ panic::catch_unwind(||{ let cb=|_l:BrokenLink<'_>|Some(("cb".into(),"t".into())); for _ in Parser::new_with_broken_link_callback(s,o,Some(cb)){} }).is_err() }
fn fails0(s:&str,o:Options)->bool{ panic::catch_unwind(||{ for _ in Parser::new_ext(s,o){} }).is_err() }
fn main(){ panic::set_hook(Box::new(|_|{}));
 let o=Options::from_bits_truncate(std::env::args().nth(1).unwrap().parse().unwrap());
 let mut s=std::env::args().nth(2).unwrap().replace("\\n","\n").replace("\\t","\t").replace("\\r","\r");
 println!("cb fails {} nocb fails {}",fails(&s,o),fails0(&s,o)); if !fails(&s,o){return;}
 loop{ let mut ch=false; let idx:Vec<usize>=s.char_indices().map(|x|x.0).collect();
   'o: for len in (1..=idx.len().min(8)).rev(){ for &i in idx.iter(){ let e=s[i..].char_indices().nth(len).map(|x|x.0+i).unwrap_or(s.len()); let mut t=s.clone(); t.replace_range(i..e,""); if fails(&t,o){s=t;ch=true;break 'o;} } }
   if !ch{break;} }
 println!("reduced {s:?} nocb fails {}",fails0(&s,o));
 for bits in [Options::ENABLE_WIKILINKS.bits()]{ let o2=Options::from_bits_truncate(bits); println!("wikilinks-only cb={} nocb={}",fails(&s,o2),fails0(&s,o2)); }
 let _=panic::take_hook(); let cb=|_l:BrokenLink<'_>|Some(("cb".into(),"t".into())); for _ in Parser::new_with_broken_link_callback(&s[..],o,Some(cb)){}
}
