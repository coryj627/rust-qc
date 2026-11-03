use pulldown_cmark::*;
pub fn opts(s: &str) -> Options {
    let mut o = Options::empty();
    for w in s.split(',') {
        o |= match w.trim() {
            "" | "none" => Options::empty(),
            "tables" => Options::ENABLE_TABLES,
            "fn" => Options::ENABLE_FOOTNOTES,
            "oldfn" => Options::ENABLE_OLD_FOOTNOTES,
            "strike" => Options::ENABLE_STRIKETHROUGH,
            "task" => Options::ENABLE_TASKLISTS,
            "smart" => Options::ENABLE_SMART_PUNCTUATION,
            "attrs" => Options::ENABLE_HEADING_ATTRIBUTES,
            "yaml" => Options::ENABLE_YAML_STYLE_METADATA_BLOCKS,
            "plus" => Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,
            "math" => Options::ENABLE_MATH,
            "gfm" => Options::ENABLE_GFM,
            "dl" => Options::ENABLE_DEFINITION_LIST,
            "sup" => Options::ENABLE_SUPERSCRIPT,
            "sub" => Options::ENABLE_SUBSCRIPT,
            "wiki" => Options::ENABLE_WIKILINKS,
            x => panic!("bad opt {x}"),
        };
    }
    o
}
pub fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' { out.push(c); continue; }
        match it.next().unwrap() {
            'n' => out.push('\n'), 'r' => out.push('\r'), 't' => out.push('\t'),
            '\\' => out.push('\\'), '0' => out.push('\0'),
            'x' => { let h: String = it.by_ref().take(2).collect(); out.push(u8::from_str_radix(&h,16).unwrap() as char); }
            'u' => { let h: String = it.by_ref().take(4).collect(); out.push(char::from_u32(u32::from_str_radix(&h,16).unwrap()).unwrap()); }
            x => { out.push('\\'); out.push(x) }
        }
    }
    out
}
pub fn html(o: Options, s: &str) -> String {
    let mut h = String::new();
    pulldown_cmark::html::push_html(&mut h, Parser::new_ext(s, o));
    h
}
