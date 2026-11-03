use pulldown_cmark::{html, Options, Parser};
fn h(s: &str, o: Options) -> String { let mut b=String::new(); html::push_html(&mut b, Parser::new_ext(s,o)); b }
fn t(s:&str,o:Options){println!("{:?} => {:?}",s,h(s,o));}
fn main(){
 let sb=Options::ENABLE_SUBSCRIPT; let su=Options::ENABLE_SUPERSCRIPT; let st=Options::ENABLE_STRIKETHROUGH;
 t("~~*a*~~ b",sb); t("~~[l](u)~~ b",sb); t("~~a~~ b",sb); t("~*a*~",sb); t("~~*a*~~ b",st); t("~*a*~ b",st);t("~~*a*~~ b",sb|st);
 t("^*a*^ b",su); t("^^*a*^^ b",su); t("~~`c`~~",sb); t("x ~~a [l](u) b~~ y\n\n[r]: /u",sb);
 t("<meta charset=x>\n*a*\n\n*b*\n",Options::empty());
 t("<source src=x>\n*a*\n",Options::empty());
 t("<param a=b>\n*a*\n",Options::empty());
 t("<search>\n*a*\n",Options::empty());
 t("a\n<meta x>\nb\n",Options::empty());
}
