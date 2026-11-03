// afe1d83: Table::raw_set / create_table_from with a nil or NaN key.
use mlua::{Lua, Value};

fn main() {
    let mode = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    // set_memory_limit returns the previous limit: 0 means no limit was set.
    if mode == "raw_set_nil_with_limit" {
        println!("previous memory limit: {:?}", lua.set_memory_limit(1 << 30));
    } else {
        println!("no set_memory_limit call made; Lua::new() default");
    }
    let table = lua.create_table().unwrap();
    let r = match mode.as_str() {
        "raw_set_nil" | "raw_set_nil_with_limit" => table.raw_set(Value::Nil, 1),
        "raw_set_nan" => table.raw_set(Value::Number(f64::NAN), 1),
        "create_table_from_nil" => lua.create_table_from([(Value::Nil, 1)]).map(|_| ()),
        _ => panic!("mode"),
    };
    println!("returned: {r:?}");
}
