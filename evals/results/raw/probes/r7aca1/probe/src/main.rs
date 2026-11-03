mod p2;
use mlua::prelude::*;
use mlua::{Function, Lua, Nil, Result, Table, Thread, Value, LuaSerdeExt, SerializeOptions};
use std::collections::{HashSet, BTreeSet};

fn main() -> Result<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "nil_key" => {
            let t = lua.create_table()?;
            println!("{:?}", t.set(Nil, 1));
        }
        "nan_key" => {
            let t = lua.create_table()?;
            println!("{:?}", t.set(f64::NAN, 1));
        }
        "raw_nil_key" => {
            let t = lua.create_table()?;
            println!("{:?}", t.raw_set(Nil, 1));
        }
        "from_nil" => {
            println!("{:?}", lua.create_table_from([(Nil, 1)]));
        }
        "cb_nil_key" => {
            // Same inside a callback called through pcall from Lua
            let t = lua.create_table()?;
            let f = lua.create_function(move |_, ()| { t.set(Nil, 1)?; Ok(()) })?;
            lua.globals().set("f", f)?;
            println!("{:?}", lua.load("return pcall(f)").eval::<LuaMultiValue>());
            println!("after");
        }
        "clear_leak" => {
            let t = lua.create_table()?;
            for i in 0..2_000_000u64 {
                if let Err(e) = t.clear() { println!("clear failed at {i}: {e}"); return Ok(()); }
            }
            println!("clear ok");
        }
        "eq_prefix" => {
            let t: Table = lua.load("return {1,2}").eval()?;
            println!("{{1,2}} == [1,2,3]: {}", t == [1,2,3]);
            let e = lua.create_table()?;
            println!("{{}} == [9]: {}", e == [9]);
        }
        "set_roundtrip" => {
            let s: HashSet<i64> = [1,2,3].into_iter().collect();
            let v = lua.pack(s)?;
            println!("{:?}", lua.unpack::<HashSet<i64>>(v.clone()));
            println!("{:?}", lua.unpack::<BTreeSet<i64>>(v));
            let s: BTreeSet<bool> = [true].into_iter().collect();
            let _ = s;
        }
        "rep_huge" => {
            println!("{:?}", lua.load("return #string.rep('x', 1<<50)").eval::<i64>());
        }
        "rep_huge_limit" => {
            lua.set_memory_limit(1<<62)?;
            println!("{:?}", lua.load("return #string.rep('x', 1<<50)").eval::<i64>());
        }
        "pairs_modify" => {
            let t = lua.create_table()?;
            for i in 0..4 { t.set(format!("k{i}"), i)?; }
            for pair in t.pairs::<Value, Value>() {
                let (k, _) = pair?;
                t.set(k, Nil)?;
                lua.gc_collect()?;
                for i in 0..1000 { t.set(format!("n{i}"), i)?; }
                lua.gc_collect()?;
            }
            println!("pairs done");
        }
        "for_each_modify" => {
            let t = lua.create_table()?;
            for i in 0..4 { t.set(format!("k{i}"), i)?; }
            let t2 = t.clone();
            let r = t.for_each(|k: Value, _: Value| {
                t2.set(k, Nil)?;
                for i in 0..1000 { t2.set(format!("n{i}"), i)?; }
                Ok(())
            });
            println!("for_each done {r:?}");
        }
        "reset_normal" => {
            lua.globals().set("reset", lua.create_function(|lua, th: Thread| {
                println!("status of normal co: {:?}", th.status());
                let f = lua.create_function(|_, ()| Ok(1))?;
                println!("reset: {:?}", th.reset(f));
                Ok(())
            })?)?;
            let r = lua.load(r#"
                local co1
                local a, b, c = "keep1", "keep2", {}
                co1 = coroutine.create(function()
                    local x, y, z = "x", "y", {1,2,3}
                    local co2 = coroutine.create(function() reset(co1) end)
                    assert(coroutine.resume(co2))
                    return x .. y .. #z
                end)
                return coroutine.resume(co1)
            "#).eval::<LuaMultiValue>();
            println!("{r:?}");
        }
        "reset_main" => {
            let main = lua.current_thread();
            println!("main status from main: {:?}", main.status());
            lua.globals().set("main_thread", main)?;
            lua.globals().set("reset", lua.create_function(|lua, th: Thread| {
                println!("status of main from co: {:?}", th.status());
                let f = lua.create_function(|_, ()| Ok(1))?;
                println!("reset: {:?}", th.reset(f));
                Ok(())
            })?)?;
            let r = lua.load(r#"
                local co = coroutine.wrap(function() reset(main_thread) end)
                co()
                return 1
            "#).eval::<i32>();
            println!("{r:?}");
        }
        "resume_normal" => {
            lua.globals().set("res", lua.create_function(|_, th: Thread| {
                println!("resume: {:?}", th.resume::<()>(()));
                Ok(())
            })?)?;
            let r = lua.load(r#"
                local co1
                co1 = coroutine.create(function()
                    local co2 = coroutine.create(function() res(co1) end)
                    assert(coroutine.resume(co2))
                    return "ok"
                end)
                return coroutine.resume(co1)
            "#).eval::<LuaMultiValue>();
            println!("{r:?}");
        }
        "sparse" => {
            let v: Value = lua.load("return {[2^40] = 1}").eval()?;
            let s = serde_json::to_string(&v.to_serializable().detect_mixed_tables(true));
            println!("{:?}", s.map(|s| s.len()));
        }
        "sparse_from" => {
            let v: Value = lua.load("return {[2^40] = 1}").eval()?;
            let opts = mlua::DeserializeOptions::new().detect_mixed_tables(true);
            let r: Result<serde_json::Value> = lua.from_value_with(v, opts);
            println!("{:?}", r.map(|_| ()));
        }
        "sparse_small" => {
            let v: Value = lua.load("return {[1000] = 1}").eval()?;
            let s = serde_json::to_string(&v.to_serializable().detect_mixed_tables(true)).unwrap();
            println!("len {}", s.len());
            let v: Value = lua.load("return {[1.5] = 1}").eval()?;
            println!("{:?}", serde_json::to_string(&v.to_serializable().detect_mixed_tables(true)));
            let v: Value = lua.load("return {[1]=1, [2]=2, x=3}").eval()?;
            println!("{:?}", serde_json::to_string(&v.to_serializable()));
            println!("{:?}", serde_json::to_string(&v.to_serializable().detect_mixed_tables(true)));
        }
        "deep_ser" => {
            let v: Value = lua.load("local t = {} for i=1,1000000 do t = {t} end return t").eval()?;
            println!("{:?}", serde_json::to_string(&v).map(|s| s.len()));
        }
        "deep_de" => {
            let v: Value = lua.load("local t = {} for i=1,1000000 do t = {t} end return t").eval()?;
            let r: Result<serde_json::Value> = lua.from_value(v);
            println!("{:?}", r.map(|_| ()));
        }
        "deep_fmt" => {
            let v: Value = lua.load("local t = {} for i=1,1000000 do t = {t} end return t").eval()?;
            println!("{}", format!("{v:#?}").len());
        }
        "deep_to_value" => {
            // deep JSON into Lua
            let mut j = serde_json::json!(1);
            for _ in 0..1000000 { j = serde_json::Value::Array(vec![j]); }
            let r = lua.to_value(&j);
            println!("{:?}", r.map(|_| ()));
            std::mem::forget(j);
        }
        other => return p2::run(other, lua),
    }
    Ok(())
}
