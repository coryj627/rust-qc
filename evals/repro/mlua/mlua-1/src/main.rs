// 98cbcd3: detect_mixed_tables encodes a one-entry table as an array up to its integer key.
use mlua::{DeserializeOptions, Lua, LuaSerdeExt, Value};
use std::time::Instant;

fn hwm_kb() -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap();
    s.lines()
        .find(|l| l.starts_with("VmHWM"))
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap()
        .parse()
        .unwrap()
}

fn main() {
    let mode = std::env::args().nth(1).unwrap();
    let key: i64 = std::env::args().nth(2).unwrap().parse().unwrap();
    let lua = Lua::new();
    let table = lua.create_table_from([(key, 42)]).unwrap();
    let value = Value::Table(table);
    let before = hwm_kb();
    let start = Instant::now();
    let out_len = match mode.as_str() {
        "ser" => serde_json::to_vec(&value.to_serializable().detect_mixed_tables(true))
            .unwrap()
            .len(),
        "de" => {
            let opts = DeserializeOptions::new().detect_mixed_tables(true);
            let v: serde_json::Value = lua.from_value_with(value.clone(), opts).unwrap();
            match v {
                serde_json::Value::Array(a) => a.len(),
                other => other.to_string().len(),
            }
        }
        _ => panic!("mode"),
    };
    println!(
        "mode={mode} table_entries=1 key={key} output_len={out_len} ms={} peak_rss_growth_kb={}",
        start.elapsed().as_millis(),
        hwm_kb() - before
    );
}
