use simd_json::prelude::*;
use simd_json::{to_owned_value, to_borrowed_value, OwnedValue};
fn t(s: &str) {
    let mut b = s.as_bytes().to_vec();
    let r = to_owned_value(&mut b);
    let sj: Result<serde_json::Value,_> = serde_json::from_str(s);
    let shown = if s.len() > 90 { format!("{}...[{}]", &s[..60], s.len()) } else { s.to_string() };
    println!("{:?}\n   simd={:?}\n   sj={:?}", shown, r, sj.map_err(|e| e.to_string()));
}
fn main() {
    // float digit_count bug
    let digits30 = "1.23456789012345678901234567890";
    for pad in [0usize, 40, 55, 60, 65, 70, 75, 78, 80, 90] {
        let s = format!("{}{}", " ".repeat(pad), digits30);
        t(&s);
    }
    t(&format!("[{}]", "1.2345678901234567890123"));
    // trailing garbage
    t("1.23456789012345678901234x");
    t("[1.23456789012345678901234x]");
    t("12345678901234567890x");
    t("[12345678901234567890x]");
    t("123456789012345678-");
    t("[1234567890123456789abc]");
    // exponent truncation
    t("1e4294967296");
    t("1e4294967297");
    t("1e-4294967296");
    t("1.5e4294967296");
    t("1e4294967295");
    t("1e2147483648");
    // surrogate
    t(r#""\ud800""#);
    t(r#""\ud800x""#);
    t(r#""\ud800A""#);
    t(r#""\ud800￿""#);
    t(r#""\udbff""#);
    t(r#""\udc00""#);
    t(r#""\uZZZZ""#);
    t(r#""😀""#);
    t(r#""abc\ud800""#);
    // NUL
    t("true\0");
    t("[true\0]");
    t("null\0");
    t("123\0");
    t("[1.5\0]");
    t("{\"a\":false\0}");
    // other
    t("-");
    t("-0");
    t("0123");
    t("01");
    t("-01");
    t("[1,]");
    t("{\"a\":1,}");
    t("[1 2]");
    t("{\"a\" 1}");
    t("{}x");
    t("[]x");
    t("[] []");
    t("\"a\" 1");
    t("");
    t("   ");
    t("]");
    t("}");
    t(":");
    t(",");
    t("{\"a\":}");
    t("[,1]");
    t("nul");
    t("tru");
    t("\"");
    t("\"\\");
    t("\"\\\"");
    t("18446744073709551615");
    t("18446744073709551616");
    t("-9223372036854775808");
    t("-9223372036854775809");
    t("999999999999999999");
    t("-999999999999999999");
    t("1e308");
    t("1.7976931348623157e308");
    t("1.7976931348623159e308");
    t("4.9e-324");
    t("2.4703282292062327e-324");
    t("1e-400");
    t("-1e-400");
    t("0e999999999999");
    t("0.0e99999999999999999999");
    t("0e4294967296000");
    let _ = to_borrowed_value;
    let _: Option<OwnedValue> = None;
    let v = to_owned_value(&mut b"1".to_vec()).unwrap();
    let _ = v.as_u8();
}
