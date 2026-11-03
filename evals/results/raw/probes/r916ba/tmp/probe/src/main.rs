use mlua::prelude::*;
use mlua::{Thread, Value, Table, Function, SerializeOptions};

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "table_eq" => {
            let t = lua.create_sequence_from([1, 2])?;
            println!("{{1,2}} == [1,2,3] -> {}", t == [1, 2, 3]);
            let e = lua.create_table()?;
            println!("{{}} == [1] -> {}", e == [1]);
        }
        "clear_leak" => {
            let t = lua.create_table()?;
            let mut n = 0u64;
            for _ in 0..2_000_000 {
                match t.clear() { Ok(()) => n += 1, Err(e) => { println!("failed after {n}: {e}"); return Ok(()); } }
            }
            println!("ok {n}");
        }
        "clear_leak_cb" => {
            let t = lua.create_table()?;
            let f = lua.create_function(move |_, ()| { t.clear()?; Ok(7) })?;
            let r: LuaResult<i32> = f.call(());
            println!("{:?}", r);
            let v: Value = lua.load("local x = 1; return x").eval()?;
            println!("{:?}", v);
        }
        "pairs_mut" => {
            let t: Table = lua.load("local t = {} for i=1,8 do t['k'..i]=i end return t").eval()?;
            let t2 = t.clone();
            let mut i = 0;
            for p in t.pairs::<String, i64>() {
                let (k, _) = p?;
                t2.raw_set(k.as_str(), Value::Nil)?;
                for _ in 0..200 { i += 1; t2.raw_set(format!("new{i}"), i)?; }
                lua.gc_collect()?;
                lua.gc_collect()?;
            }
            println!("finished iteration");
        }
        "for_each_mut" => {
            let t: Table = lua.load("local t = {} for i=1,8 do t['k'..i]=i end return t").eval()?;
            let t2 = t.clone();
            let mut i = 0;
            let r = t.for_each::<String, i64>(|k, _| {
                t2.raw_set(k.as_str(), Value::Nil)?;
                for _ in 0..200 { i += 1; t2.raw_set(format!("new{i}"), i)?; }
                Ok(())
            });
            println!("finished {:?}", r);
        }
        "resume_main" => {
            let f = lua.create_function(|_, t: Thread| {
                println!("status of main from coroutine: {:?}", t.status());
                let r = t.resume::<()>(());
                println!("resume -> {:?}", r.as_ref().map_err(|e| e.to_string()));
                Ok(())
            })?;
            lua.globals().set("rf", f)?;
            let r: LuaResult<i64> = lua.load(r#"
                local a, b = 11, 22
                local main = coroutine.running()
                local co = coroutine.create(function() rf(main) end)
                print(coroutine.resume(co))
                return a + b
            "#).eval();
            println!("{:?}", r);
        }
        "resume_normal_rust" => {
            let f = lua.create_function(|_, t: Thread| {
                println!("status: {:?}", t.status());
                let r = t.resume::<()>(());
                println!("resume -> {:?}", r.as_ref().map_err(|e| e.to_string()));
                Ok(())
            })?;
            lua.globals().set("rf", f)?;
            let co: Thread = lua.load(r#"
              coroutine.create(function()
                local a, b = 11, 22
                local me = coroutine.running()
                local inner = coroutine.create(function() rf(me) end)
                print(coroutine.resume(inner))
                print('after', a, b)
                return a + b
              end)
            "#).eval()?;
            let r: LuaResult<i64> = co.resume(());
            println!("{:?}", r);
        }
        "reset_normal" => {
            let f = lua.create_function(|lua, t: Thread| {
                println!("status: {:?}", t.status());
                let r = t.reset(lua.create_function(|_, ()| Ok(()))?);
                println!("reset -> {:?}", r.as_ref().map_err(|e| e.to_string()));
                Ok(())
            })?;
            lua.globals().set("rf", f)?;
            let r: LuaResult<i64> = lua.load(r#"
                local a, b = 11, 22
                local main = coroutine.running()
                local co = coroutine.create(function() rf(main) end)
                print(coroutine.resume(co))
                print(a, b)
                return a + b
            "#).eval();
            println!("{:?}", r);
        }
        "close_err" => {
            let co: Thread = lua.load(r#"
              coroutine.create(function()
                local x <close> = setmetatable({}, {__close=function() print('close ran'); error('boom in close') end})
                error('first')
              end)
            "#).eval()?;
            let r: LuaResult<()> = co.resume(());
            println!("resume -> {:?}", r.map_err(|e| e.to_string()));
            println!("survived; status {:?}", co.status());
        }
        "close_err_cb" => {
            let co: Thread = lua.load(r#"
              coroutine.create(function()
                local x <close> = setmetatable({}, {__close=function() print('close ran') end})
                error('first')
              end)
            "#).eval()?;
            let r: LuaResult<()> = co.resume(());
            println!("resume -> {:?}", r.map_err(|e| e.to_string()));
            println!("status {:?}", co.status());
            let f = lua.create_function(|_, ()| Ok(()))?;
            println!("reset -> {:?}", co.reset(f).map_err(|e| e.to_string()));
        }
        "reset_err" => {
            for code in ["error(true)", "error('msg')", "error(42)", "error({})"] {
                let f = lua.load(code).into_function()?;
                let co = lua.create_thread(f)?;
                let r: LuaResult<()> = co.resume(());
                println!("{code}: resume -> {:?}", r.map_err(|e| e.to_string()));
                let g = lua.create_function(|_, ()| Ok(()))?;
                println!("{code}: reset -> {:?}", co.reset(g).map_err(|e| e.to_string()));
            }
        }
        "sparse" => { let t: Value = lua.load("return {[1e12]=1}").eval()?; let s = t.to_serializable().detect_mixed_tables(true); let j = serde_json::to_string(&s); println!("{:?}", j.map(|x| x.len())); }
        _ => panic!("unknown"),
    }
    let _ = (SerializeOptions::new(), 0 as i64 as i32);
    let _: Option<Function> = None;
    Ok(())
}
