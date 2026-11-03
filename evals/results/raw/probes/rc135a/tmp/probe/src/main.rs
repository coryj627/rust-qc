use mlua::prelude::*;
use mlua::{Function, Nil, Table, Thread, UserData, UserDataMethods, Value};

struct Payload { name: String }
impl UserData for Payload {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("name", |_, this, ()| { println!("in method, ptr={:p} len={}", this.name.as_ptr(), this.name.len()); Ok(this.name.clone()) });
    }
}
struct PanicDrop;
impl UserData for PanicDrop {}
impl Drop for PanicDrop { fn drop(&mut self) { panic!("drop panic"); } }

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "table_eq" => {
            let t = lua.create_table()?;
            println!("empty == [1,2,3] -> {}", t == [1, 2, 3]);
            let t: Table = lua.load("{1,2}").eval()?;
            println!("{{1,2}} == [1,2,3] -> {}", t == [1, 2, 3]);
        }
        "nil_key" => {
            let t = lua.create_table()?;
            println!("set nil key: {:?}", t.set(Nil, 1));
        }
        "nan_key" => {
            let t = lua.create_table()?;
            println!("set nan key: {:?}", t.raw_set(f64::NAN, 1));
        }
        "nil_key_callback" => {
            let f = lua.create_function(|_, (t, k): (Table, Value)| { t.set(k, 1)?; Ok(()) })?;
            lua.globals().set("rust_set", f)?;
            println!("{:?}", lua.load("return pcall(rust_set, {}, nil)").eval::<LuaMultiValue>());
            println!("after");
        }
        "table_from_nil" => {
            println!("{:?}", lua.create_table_from(vec![(Nil, 1)]));
        }
        "mem_limit_nil" => {
            lua.set_memory_limit(10_000_000)?;
            let t = lua.create_table()?;
            println!("with limit: {:?}", t.set(Nil, 1));
        }
        "clear_leak" => {
            let t = lua.create_table()?;
            for i in 0..2_000_000u32 {
                if let Err(e) = t.clear() { println!("clear failed at iter {i}: {e}"); break; }
            }
            println!("create_table after: {:?}", lua.create_table().map(|_| "ok"));
            println!("done");
        }
        "mixed_sparse" => {
            let t: Value = lua.load("{[50000000]=true}").eval()?;
            let s = serde_json::to_string(&t.to_serializable().detect_mixed_tables(true)).unwrap();
            println!("len {} head {}", s.len(), &s[..40.min(s.len())]);
        }
        "mixed_empty" => {
            let t: Value = lua.load("{}").eval()?;
            println!("{}", serde_json::to_string(&t.to_serializable().detect_mixed_tables(true)).unwrap());
        }
        "cross_mt" => {
            let lua2 = Lua::new();
            // same ref index holds a string in lua and a table in lua2
            let s = lua.create_string("not a table")?;
            let a = lua.create_table()?;
            let mts: Vec<Table> = (0..4).map(|_| lua2.create_table().unwrap()).collect();
            let _ = &s;
            // choose metatable in lua2 whose index lines up with `s` in lua
            for (i, mt) in mts.iter().enumerate() {
                println!("try {i}");
                a.set_metatable(Some(mt.clone()))?;
                println!(" mt set; has_metatable={}", a.has_metatable());
                println!(" a.metatable type ok={:?}", a.metatable().map(|m| m.raw_len()));
                println!(" index: {:?}", a.get::<Value>("zzz"));
            }
        }
        "cross_thread" => {
            let lua2 = Lua::new();
            let s = lua.create_string("nope")?;
            let f2: Function = lua2.load("return function() return 2 end").eval()?;
            let th = lua.create_thread(f2.clone());
            println!("{:?} {:?}", th.as_ref().map(|t| t.status()), s);
            println!("{:?}", th.and_then(|t| t.resume::<Value>(())));
        }
        "pairs_mutate" => {
            let t: Table = lua.load("{a=1,b=2,c=3,d=4}").eval()?;
            for kv in t.pairs::<Value, Value>() {
                let (k, _) = kv?;
                t.set(k, Nil)?;
                for i in 0..100 { t.set(format!("x{i}"), 1)?; }
            }
            println!("finished");
        }
        "for_each_mutate" => {
            let t: Table = lua.load("{a=1,b=2,c=3,d=4}").eval()?;
            t.clone().for_each(|k: Value, _v: Value| {
                t.set(k, Nil)?;
                for i in 0..100 { t.set(format!("x{i}"), 1)?; }
                Ok(())
            })?;
            println!("finished");
        }
        "scope_uaf" => {
            lua.scope(|s| {
                let ud = s.create_userdata(Payload { name: "hello world, long enough string".into() })?;
                lua.globals().set("ud", ud)?;
                lua.load("saved = ud.name").exec()?;
                println!("scope call: {:?}", lua.load("return saved(ud)").eval::<String>());
                Ok(())
            })?;
            // allocate to reuse the freed box
            let junk: Vec<Vec<u8>> = (0..64).map(|_| vec![0x41u8; 32]).collect();
            println!("after scope normal method: {:?}", lua.load("return ud:name()").eval::<String>().map_err(|e| e.to_string()));
            println!("after scope saved method: {:?}", lua.load("return saved(ud)").eval::<LuaString>().map(|s| s.as_bytes().len()).map_err(|e| e.to_string()));
            drop(junk);
        }
        "scope_any_uaf" => {
            lua.scope(|s| {
                let ud = s.create_any_userdata(String::from("hello world, long enough string"), |r| {
                    r.add_method("name", |_, this, ()| Ok(this.clone()));
                    r.add_method_mut("clobber", |_, this, ()| { this.push('x'); Ok(()) });
                })?;
                lua.globals().set("ud", ud)?;
                lua.load("saved = ud.name; savedm = ud.clobber").exec()?;
                Ok(())
            })?;
            println!("saved: {:?}", lua.load("return saved(ud)").eval::<LuaString>().map(|s| s.as_bytes().len()).map_err(|e| e.to_string()));
            println!("savedm: {:?}", lua.load("return savedm(ud)").exec().map_err(|e| e.to_string()));
        }
        "panic_drop" => {
            lua.globals().set("x", PanicDrop)?;
            lua.load("x = nil").exec()?;
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lua.gc_collect()));
            println!("gc_collect -> {:?}", r.as_ref().map(|r| r.is_ok()).map_err(|_| "panicked"));
            println!("used before {}", lua.used_memory());
            println!("eval: {:?}", lua.load("local t = {} for i=1,1e5 do t[i]={} end return #t").eval::<i64>());
            println!("gc_is_running: {}", lua.gc_is_running());
            println!("pcall: {:?}", lua.load("return pcall(error, 'x')").eval::<LuaMultiValue>().map(|v| v.len()));
            println!("err: {:?}", lua.load("error('boom')").exec().map_err(|e| e.to_string()));
            println!("create_table_from nil:{:?}", lua.create_table_from(vec![(Nil,1)]).is_ok());
            println!("end");
        }
        "resume_normal" => {
            let f = lua.create_function(|_, th: Thread| {
                println!("status of outer seen from inner: {:?}", th.status());
                let r = th.resume::<Value>(());
                println!("resume -> {:?}", r.map_err(|e| e.to_string()));
                Ok(())
            })?;
            lua.globals().set("rust_resume", f)?;
            let r: LuaResult<Value> = lua.load(r#"
                local outer
                local function rust_mid(...) return coroutine.resume(coroutine.create(function() rust_resume(outer) end)) end
                outer = coroutine.create(function(a,b,c)
                    local x, y = 10, 20
                    print("inner", rust_mid(1,2,3))
                    return x + y
                end)
                return select(2, coroutine.resume(outer, 1, 2, 3))
            "#).eval();
            println!("{:?}", r);
        }
        "resume_normal_cb" => {
            // A Rust callback on the outer coroutine resumes inner, whose callback resumes outer
            let g = lua.create_function(|_, th: Thread| {
                println!("status: {:?}", th.status());
                let r = th.resume::<Value>(());
                println!("resume -> {:?}", r.map_err(|e| e.to_string()));
                Ok(())
            })?;
            lua.globals().set("g", g)?;
            let f = lua.create_function(|_, (co, _a, _b): (Thread, i32, i32)| {
                let r = co.resume::<Value>(());
                println!("mid resume -> {:?}", r.map_err(|e| e.to_string()));
                Ok(42)
            })?;
            lua.globals().set("f", f)?;
            let r: LuaResult<Value> = lua.load(r#"
                local outer
                outer = coroutine.create(function()
                    local inner = coroutine.create(function() g(outer) end)
                    return f(inner, 1, 2)
                end)
                return select(2, coroutine.resume(outer))
            "#).eval();
            println!("{:?}", r);
        }
        "reset_normal" => {
            let g = lua.create_function(|lua, th: Thread| {
                println!("status {:?}", th.status());
                let nf = lua.create_function(|_, ()| Ok(()))?;
                println!("reset -> {:?}", th.reset(nf).map_err(|e| e.to_string()));
                Ok(())
            })?;
            lua.globals().set("g", g)?;
            let f = lua.create_function(|_, co: Thread| {
                let r = co.resume::<Value>(());
                println!("mid resume -> {:?}", r.map_err(|e| e.to_string()));
                Ok(42)
            })?;
            lua.globals().set("f", f)?;
            let r: LuaResult<Value> = lua.load(r#"
                local outer
                outer = coroutine.create(function()
                    local inner = coroutine.create(function() g(outer) end)
                    return f(inner)
                end)
                return select(2, coroutine.resume(outer))
            "#).eval();
            println!("{:?}", r);
        }
        "reset_error" => {
            let f: Function = lua.load("return function() error('first') end").eval()?;
            let th = lua.create_thread(f.clone())?;
            println!("resume {:?}", th.resume::<()>(()).map_err(|e| e.to_string()));
            println!("status {:?}", th.status());
            println!("reset {:?}", th.reset(f.clone()).map_err(|e| e.to_string()));
            println!("status {:?}", th.status());
            println!("resume {:?}", th.resume::<()>(()).map_err(|e| e.to_string()));
        }
        "pretty_shared" => {
            let t: Value = lua.load("local s = {1}; return {a=s, b=s}").eval()?;
            println!("{:#?}", t);
        }
        "catch_panics_yield" => {
            let lua = unsafe { Lua::unsafe_new_with(mlua::StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(false)) };
            let r = lua.load(r#"
              local co = coroutine.wrap(function() return pcall(function() coroutine.yield(1); return 2 end) end)
              return co(), co()
            "#).eval::<LuaMultiValue>();
            println!("{:?}", r.map_err(|e| e.to_string()));
        }
        "err_bool" => {
            let f: Function = lua.load("return function() error(true) end").eval()?;
            println!("{:?}", f.call::<()>(()).map_err(|e| e.to_string()));
            let th = lua.create_thread(f.clone())?;
            println!("{:?}", th.resume::<()>(()).map_err(|e| e.to_string()));
            println!("reset {:?}", th.reset(f.clone()).map_err(|e| e.to_string()));
        }
        _ => panic!("unknown"),
    }
    Ok(())
}
