// 6ec314a: no depth limit on serde traversal of a Lua table built by a script.
use mlua::{Lua, LuaSerdeExt, Value};

fn main() {
    let mode = std::env::args().nth(1).unwrap();
    let depth: i64 = std::env::args().nth(2).unwrap().parse().unwrap();
    let lua = Lua::new();
    let value: Value = lua
        .load("local depth = ...; local t = {}; for i = 1, depth do t = {next = t} end; return t")
        .call(depth)
        .unwrap();
    println!("built depth={depth}");
    match mode.as_str() {
        "ser" => {
            let r = serde_json::to_vec(&value);
            println!("returned: {:?}", r.map(|v| v.len()).map_err(|e| e.to_string()));
        }
        "de" => {
            let r = lua.from_value::<serde_json::Value>(value);
            println!("deserialized, is_ok={}", r.is_ok());
            if let Err(e) = &r {
                println!("error: {e}");
            }
            // serde_json::Value's Drop also recurses: forget it, so any overflow seen here
            // comes from the traversal in mlua and not from dropping the result.
            std::mem::forget(r);
        }
        _ => panic!("mode"),
    }
}
