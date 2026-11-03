use mlua::prelude::*;
use mlua::{Table, Value};
use std::rc::Rc;

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "err_key_pairs" => {
            lua.globals().set("bad", lua.create_function(|_, ()| Err::<(), _>(LuaError::runtime("x")))?)?;
            let t: Table = lua.load("local ok, e = pcall(bad); return {[e] = 1, x = 2, y = 3}").eval()?;
            let mut n = 0;
            for p in t.pairs::<Value, Value>() { let _ = p?; n += 1; println!("pair {n}"); }
            println!("survived {n}");
        }
        "err_key_json" => {
            lua.globals().set("bad", lua.create_function(|_, ()| Err::<(), _>(LuaError::runtime("x")))?)?;
            let t: Value = lua.load("local ok, e = pcall(bad); return {[e] = 1, x = 2, y = 3}").eval()?;
            let r: LuaResult<std::collections::HashMap<String, i32>> = lua.from_value_with(t, mlua::DeserializeOptions::new().deny_unsupported_types(false));
            println!("{:?}", r.map_err(|e| e.to_string()));
        }
        "finalizer_handle_close" => {
            let f = lua.create_function(|lua, ()| { println!("finalizer start"); lua.globals().set("x", 1)?; println!("finalizer end"); Ok(()) })?;
            lua.globals().set("fin", f)?;
            lua.load("keep = setmetatable({}, {__gc = fin})").exec()?;
            drop(lua);
            println!("dropped");
            return Ok(());
        }
        "warn_co" => {
            lua.set_warning_function(|lua, msg, _| { println!("warn {msg} on {:?}", lua.current_thread().status()); Err(LuaError::runtime("warning error")) });
            let r: LuaResult<(bool, String)> = lua.load(r#"
                local co = coroutine.create(function() warn("x"); return 1 end)
                local a, b = coroutine.resume(co)
                return a, tostring(b) .. " / " .. coroutine.status(co)
            "#).eval();
            println!("{:?}", r.map_err(|e| e.to_string().lines().next().unwrap().to_string()));
            println!("alive {:?}", lua.load("return 1").eval::<i32>().map_err(|e| e.to_string()));
        }
        "depth" => {
            for d in [1000, 5000, 10000, 20000, 50000, 100000, 200000] {
                let t: Value = lua.load(format!("local t = {{}} for i=1,{d} do t = {{t}} end return t")).eval()?;
                let r = serde_json::to_string(&t).map(|s| s.len()).map_err(|e| e.to_string());
                println!("depth {d}: {:?}", r);
            }
        }
        "oom_leak" => {
            let token = Rc::new(());
            let t2 = token.clone();
            lua.set_memory_limit(1)?;
            let r = lua.create_function(move |_, ()| { let _ = &t2; Ok(()) });
            println!("create_function -> {:?}", r.as_ref().map(|_| ()).map_err(|e| e.to_string().lines().next().unwrap().to_string()));
            drop(r);
            lua.set_memory_limit(0)?;
            lua.gc_collect()?; lua.gc_collect()?;
            println!("strong count after failure: {}", Rc::strong_count(&token));
        }
        "array_mt_mixed" => {
            lua.globals().set("array_mt", lua.array_metatable())?;
            let t: Value = lua.load("setmetatable({a = 5}, array_mt)").eval()?;
            println!("default: {}", serde_json::to_string(&t.to_serializable()).unwrap());
            println!("mixed:   {}", serde_json::to_string(&t.to_serializable().detect_mixed_tables(true)).unwrap());
        }
        "status_normal" => {
            lua.globals().set("rs", lua.create_function(|lua, t: mlua::Thread| {
                let lua_says: String = lua.load("return coroutine.status(...)").call(t.clone())?;
                println!("mlua status: {:?}, Lua status: {lua_says}", t.status());
                Ok(())
            })?)?;
            lua.load("local me = coroutine.running(); coroutine.wrap(function() rs(me) end)()").exec()?;
            lua.load("coroutine.wrap(function() local me = coroutine.running(); coroutine.wrap(function() rs(me) end)() end)()").exec()?;
        }
        _ => panic!(),
    }
    Ok(())
}
