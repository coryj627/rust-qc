// 3771a4d: Value == casts Integer to f64 before comparing with Number.
use mlua::{Lua, Result, Value};

fn main() -> Result<()> {
    let lua = Lua::new();
    let cases = [
        (i64::MAX, i64::MAX as f64, "math.maxinteger == 2^63"),
        ((1i64 << 53) + 1, (1u64 << 53) as f64, "(1 << 53) + 1 == 2^53"),
        (1i64 << 53, (1u64 << 53) as f64, "(1 << 53) == 2^53"),
    ];
    let mut wrong = 0;
    for (i, f, expr) in cases {
        let (a, b) = (Value::Integer(i), Value::Number(f));
        let rust_ab = a == b;
        let rust_ba = b == a;
        let lua_says: bool = lua.load(format!("return {expr}")).eval()?;
        let flag = if rust_ab == lua_says && rust_ba == lua_says { "ok" } else { wrong += 1; "WRONG" };
        println!("{expr}: Value==: {rust_ab}/{rust_ba}, Lua ==: {lua_says}: {flag}");
    }
    println!("wrong results: {wrong}");
    std::process::exit(if wrong > 0 { 42 } else { 0 });
}
