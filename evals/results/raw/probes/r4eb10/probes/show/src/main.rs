use pulldown_cmark::*;
use std::panic;
fn show(name:&str,o:Options,s:&str){
  let s2=s.to_string();
  let r=panic::catch_unwind(move||{ let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(&s2,o)); h});
  match r { Ok(h)=>println!("{:<26} {:?}\n   => {:?}",name,s,h), Err(_)=>println!("{:<26} {:?}\n   => PANIC",name,s) }
}
fn main(){
  panic::set_hook(Box::new(|i|{ println!("   panic at {}: {}",i.location().unwrap(),i); }));
  let e=Options::empty();
  let all=Options::all()-Options::ENABLE_OLD_FOOTNOTES;
  // panics
  show("empty-par-tight",e,"- [a]:-\n\t  ");
  show("empty-par-ff",e,"- [a]:u\n\u{c}");
  show("empty-par-toplevel",e,"[a]: /u\n   ");
  show("empty-par-toplevel-ff",e,"[a]: /u\n\u{c}");
  show("cr-html",e,"<pre>\rx");
  show("cr-html2",e,"<div>\ra\rb");
  // CR line endings
  show("cr-fence",e,"~~~\rfoo\r~~~\r");
  show("cr-fence-bt",e,"```\rfoo\r```\r");
  show("cr-indent",e,"    a\r    b\r");
  show("crlf-codespan",e,"`a\r\nb`");
  show("lf-codespan",e,"`a\nb`");
  show("crlf-codespan-bq",e,"> `a\r\n> b`");
  show("crlf-math",Options::ENABLE_MATH,"$a\r\nb$");
  // fences and headings
  show("fence-close-tab",e,"```\nfoo\n```\t\nbar\n");
  show("info-tab",e,"``` rust\tx\nfoo\n```\n");
  show("info-tab2",e,"~~~rust\tx\nfoo\n~~~\n");
  show("atx-trailing-tab",e,"# foo\t\n");
  show("atx-closer-tab",e,"# foo #\t\n");
  show("atx-closer-pre-tab",e,"# foo\t#\n");
  show("atx-ff",e,"#\u{c}foo\n");
  show("blank-ff",e,"foo\n\u{c}\nbar\n");
  show("atx-bs-attr",Options::ENABLE_HEADING_ATTRIBUTES,"# foo\\\nbar\n");
  show("atx-bs-noattr",e,"# foo\\\nbar\n");
  show("atx-bs-eof",e,"# foo\\");
  // link labels
  let long="a".repeat(1000); show("label-1000-ascii",e,&format!("[{l}]\n\n[{l}]: /u\n",l=long));
  let long="é".repeat(600); show("label-600-2byte",e,&format!("[{l}]\n\n[{l}]: /u\n",l=long));
  // sub/sup
  let ss=Options::ENABLE_SUPERSCRIPT|Options::ENABLE_SUBSCRIPT;
  show("sup-after-tilde",ss,"^a~ b^");
  show("sup-plain",ss,"^a b^");
  show("sub-after-caret",ss,"~a^ b~");
  show("sub-only-dbl",Options::ENABLE_SUBSCRIPT,"~~*a*~~");
  show("sub-only-dbl-link",Options::ENABLE_SUBSCRIPT,"~~[a](/u)~~");
  show("sup-only",Options::ENABLE_SUPERSCRIPT,"^*a*^");
  show("sup-dbl",Options::ENABLE_SUPERSCRIPT,"^^a^^");
  show("strike-triple",Options::ENABLE_STRIKETHROUGH,"~~~a~~~");
  // tables
  let t=Options::ENABLE_TABLES;
  show("tbl-title-pipe",t,"|h|\n|-|\n|[a](/u \"x\\\\|!\")|\n");
  show("tbl-title-pipe2",t,"|h|\n|-|\n|[a](/u \"x\\|!\")|\n");
  show("tbl-delim-mid-colon",t,"a|b\n-:-|-\n");
  show("tbl-delim-space",t,"a|b\n- -|-\n");
  show("tbl-delim-last-colon",t,"a|b\n-|:\n");
  show("tbl-delim-tab",t,"a|b\n-\t|-\n");
  show("tbl-delim-dblcolon",t,"a|b\n::-|-\n");
  show("tbl-code-pipe",t,"|h|\n|-|\n|`a\\|b`|\n");
  show("tbl-wiki-pipe",t|Options::ENABLE_WIKILINKS,"|h|\n|-|\n|[[a\\|b]]|\n");
  show("tbl-refdef-pipe",t,"[a|b]: /u\n\n|h|\n|-|\n|[a\\|b]|\n");
  // wikilinks
  let w=Options::ENABLE_WIKILINKS;
  show("wiki-empty-pothole",w,"[[a|]]");
  show("wiki-multiline-bq",w,"> [[a\n> b]]");
  show("wiki-img-reuse",w,"![[a]] b](/u)");
  show("wiki-escape",w,"[[a\\]b]]");
  show("wiki-code",w,"[[`a`]]");
  show("wiki-after",w,"[[a]]*b*");
  // footnotes
  let f=Options::ENABLE_FOOTNOTES;
  show("fn-in-link",f,"[foo [^1] bar](/u)\n\n[^1]: x\n");
  show("fn-in-image",f,"![foo [^1] bar](/u)\n\n[^1]: x\n");
  show("fn-after-open",f,"[x [^1]\n\n[^1]: y\n");
  show("fn-undefined",f,"[^1]");
  show("fn-unicase",f,"[^A]\n\n[^a]: y\n");
  show("fn-num-img",f,"[^b] ![x[^a]](u)\n\n[^a]: y\n[^b]: z\n");
  // html renderer
  show("meta-inline",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,"---\na: `b` <i>\n---\nc");
  show("meta-bq",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,"> ---\n> a: 1\n> ---\n> text\n");
  show("meta-mid",Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,"foo\n\n---\na: 1\n---\nbar\n");
  show("alt-softbreak",e,"![a\nb](/u)");
  show("alt-code",e,"![`<a>`](/u)");
  show("math-html",Options::ENABLE_MATH,"$<a>$ $$<b>$$");
  show("attr-escape",Options::ENABLE_HEADING_ATTRIBUTES,"# a {#\"x k=\"v\" .\"c}\n");
  show("attr-noval",Options::ENABLE_HEADING_ATTRIBUTES,"# a {k k2= #id #id2 . # }\n");
  show("attr-inner-brace",Options::ENABLE_HEADING_ATTRIBUTES,"# a {x} {#y}\n");
  show("attr-setext",Options::ENABLE_HEADING_ATTRIBUTES,"a {#x}\nb {#y}\n===\n");
  show("attr-setext-bs",Options::ENABLE_HEADING_ATTRIBUTES,"\\* {#x}\n===\n");
  show("setext-trail-bs",e,"a\\\n===\n");
  show("setext-esc-attr",Options::ENABLE_HEADING_ATTRIBUTES,"\\{#x}\n===\n");
  show("attr-only",Options::ENABLE_HEADING_ATTRIBUTES,"{#x}\n===\n");
  show("attr-only-atx",Options::ENABLE_HEADING_ATTRIBUTES,"# {#x}\n");
  show("attr-closer",Options::ENABLE_HEADING_ATTRIBUTES,"# a # {#x}\n");
  // lists & misc
  show("list-99",e,"999999999. a\n");
  show("task-only",Options::ENABLE_TASKLISTS,"- [ ]\n- [x]   \n");
  show("task-para2",Options::ENABLE_TASKLISTS,"- [ ] a\n\n  [ ] b\n");
  show("all-dl",all,"a\n: b\n\nc\n: d\n");
  show("dl-table",all,"a\n: |b|\n  |-|\n");
  show("smart",Options::ENABLE_SMART_PUNCTUATION,"'a' \"b\" 'c\" ... -- --- ----- it's");
  show("smart-unmatched",Options::ENABLE_SMART_PUNCTUATION,"\"a\" b\" 'x");
  show("smart-in-link-title",Options::ENABLE_SMART_PUNCTUATION,"[a](/u \"t\") <a href=\"x\">");
  show("autolink-cr",e,"<http://a.b/c\rd>");
  show("ent-hex",e,"&#xD800; &#0; &#1234567; &#x110000;");
  show("title-newline-bq",e,"> [a](/u \"x\n> y\")");
  show("title-paren",e,"[a](/u (x(y)))");
  show("title-entity-esc",e,"[a](/u \"&amp;\\&\")");
  show("gfm-note-lazy",Options::ENABLE_GFM,"> [!NOTE]\nfoo\n");
  show("gfm-note-case",Options::ENABLE_GFM,"> [!note]\n> foo\n");
  show("gfm-note-blank",Options::ENABLE_GFM,"> [!NOTE]\n");
}
