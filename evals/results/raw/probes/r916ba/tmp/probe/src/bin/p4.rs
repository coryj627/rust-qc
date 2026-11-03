use mlua::prelude::*;
use mlua::{Table, Value, HookTriggers, VmState, Thread, LuaOptions, StdLib};

struct P;
impl Drop for P { fn drop(&mut self) { panic!("panic in Drop"); } }

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "hook_yield_gc" => {
            let f = lua.load(r#"
                local t = {1, 2, 3}
                local s = {name = "alive"}
                local a = t[1]
                local b = s.name
                return a, b, t, s
            "#).into_function()?;
            let co = lua.create_thread(f)?;
            co.set_hook(HookTriggers::EVERY_LINE, |_, _| Ok(VmState::Yield))?;
            let mut n = 0;
            while co.status() == mlua::ThreadStatus::Resumable {
                n += 1;
                let r = co.resume::<LuaMultiValue>(());
                println!("resume #{n}: {:?}", r.map(|v| v.len()).map_err(|e| e.to_string().lines().next().unwrap().to_string()));
                lua.gc_collect()?; lua.gc_collect()?;
                if n > 20 { break; }
            }
            println!("final status {:?}", co.status());
        }
        "pairs_err" => {
            let t: Table = lua.load(r#"local t = {} for i=1,10 do t["k"..i]=i end t.k5="not a number" return t"#).eval()?;
            let (mut ok, mut err) = (0, 0);
            for p in t.pairs::<String, i32>() { match p { Ok(_) => ok += 1, Err(_) => err += 1 } }
            let mut total = 0; t.for_each::<Value, Value>(|_, _| { total += 1; Ok(()) })?;
            println!("pairs: ok={ok} err={err} of {total} entries");
            let seq: Table = lua.load(r#"return {1,"x",3,"y",5}"#).eval()?;
            let (mut ok, mut err) = (0, 0);
            for p in seq.sequence_values::<i32>() { match p { Ok(_) => ok += 1, Err(_) => err += 1 } }
            println!("sequence_values: ok={ok} err={err}");
        }
        "drop_panic_gc" => {
            {
                let ud = lua.create_any_userdata(P)?;
                drop(ud);
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { lua.gc_collect()?; lua.gc_collect() }));
            println!("gc_collect -> {:?}", r.as_ref().map(|r| r.is_ok()).map_err(|_| "panicked"));
            println!("gc running: {}", lua.gc_is_running());
            let before = lua.used_memory();
            for _ in 0..20000 { let _ = lua.create_table()?; }
            lua.gc_collect()?;
            println!("used memory {} -> {} ", before, lua.used_memory());
            let r: LuaResult<i32> = lua.load("return 1 + 1").eval();
            println!("eval -> {:?}", r);
            let r: LuaResult<()> = lua.load("error('x')").exec();
            println!("error -> {:?}", r.map_err(|e| e.to_string().lines().next().unwrap().to_string()));
        }
        "drop_panic_take_gc_call" => {
            let f = lua.create_function(|lua, ()| { 
                let ud = lua.create_any_userdata(P)?; drop(ud);
                lua.gc_collect()?; lua.gc_collect()?; println!("after gc in cb"); Ok(()) })?;
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.call::<()>(())));
            println!("call -> {:?}", r.map(|r| r.map_err(|e| e.to_string())).map_err(|_| "panicked"));
            println!("still alive: {:?}", lua.load("return 2").eval::<i32>());
        }
        "safe_pcall_yield" => {
            let safe = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(false))?;
            let code = r#"
                local co = coroutine.wrap(function() return pcall(function() coroutine.yield(1); return 2 end) end)
                return tostring(co()), tostring(select(1, pcall(co)))
            "#;
            println!("catch_rust_panics=true : {:?}", lua.load(code).eval::<(String, String)>().map_err(|e| e.to_string()));
            println!("catch_rust_panics=false: {:?}", safe.load(code).eval::<(String, String)>().map_err(|e| e.to_string()));
        }
        "yield_stack" => {
            // yield values larger than stack
            let co: Thread = lua.load("coroutine.create(function() coroutine.yield(table.unpack({}, 1, 100000)) end)").eval()?;
            println!("{:?}", co.resume::<LuaMultiValue>(()).map(|v| v.len()).map_err(|e| e.to_string()));
        }
        "fmt_nan_sort" => {
            let v: Value = lua.load("return {[1]=0/0, [2.5]=1, a=2, [true]=3}").eval()?;
            println!("{:#?}", v);
        }
        "gc_inc_in_finalizer" => {
            let f = lua.create_function(|lua, ()| { lua.gc_gen(0, 0); Ok(()) })?;
            lua.globals().set("fin", f)?;
            lua.load("setmetatable({}, {__gc = fin}); collectgarbage(); collectgarbage()").exec()?;
            println!("ok");
        }
        _ => panic!(),
    }
    Ok(())
}
