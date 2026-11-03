// second batch of probes
use mlua::prelude::*;
use mlua::{AnyUserData, Lua, Result, Table, Thread, Value, LuaSerdeExt, MultiValue, String as LuaString};

struct Victim(String);
struct Other<const N: usize>(u64, u64, u64);

fn other<const N: usize>(lua: &Lua, hits: &mut Vec<AnyUserData>) -> Result<()> {
    let ud = lua.create_any_userdata(Other::<N>(0x4141414141414141, 0x4141414141414141, 0x4141414141414141))?;
    hits.push(ud);
    Ok(())
}

pub fn run(which: &str, lua: Lua) -> Result<()> {
    match which {
        "mt_reuse" => {
            // The check touches the fast-path cache, so only one candidate is tested per run.
            let k: usize = std::env::var("K").unwrap().parse().unwrap();
            lua.register_userdata_type::<Victim>(|r| { r.add_method("x", |_, _, ()| Ok(1)); })?;
            let v = lua.create_any_userdata(Victim("hello".into()))?;
            println!("victim borrow {:?}", v.borrow::<Victim>().map(|s| s.0.clone()));
            drop(v);
            lua.register_userdata_type::<Victim>(|_| {})?;
            lua.gc_collect()?; lua.gc_collect()?;
            let mut hits = vec![];
            macro_rules! go { ($($n:literal)*) => { $( other::<$n>(&lua, &mut hits)?; )* } }
            go!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31
                32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63);
            match hits[k].borrow::<Victim>() {
                Ok(r) => { println!("HIT {k}: borrow::<Victim> succeeded on Other<{k}>"); println!("len {:#x}", r.0.len()); println!("{:?}", r.0.as_bytes().first()); }
                Err(e) => println!("miss {k}: {e}"),
            }
        }
        "yield_nested" => {
            async fn pingpong(lua: Lua, mut val: i32) -> Result<()> {
                loop {
                    let got: MultiValue = lua.yield_with(val).await?;
                    println!("async got {:?}", got);
                    val = got.get(got.len().saturating_sub(1)).and_then(|v| v.as_i32()).unwrap_or(-1);
                }
            }
            let co = lua.create_thread(lua.create_async_function(pingpong)?)?;
            println!("first {:?}", co.resume::<i32>(1));
            println!("second top-level {:?}", co.resume::<i32>(2));
            // now resume from inside a Rust callback: pool slot is held by the callback
            let f = lua.create_function(|_, co: Thread| {
                let r = co.resume::<MultiValue>(3);
                println!("nested resume {:?}", r);
                Ok(())
            })?;
            println!("cb {:?}", f.call::<()>(&co));
            // now trigger errors so a pool slot gets used
            let e = lua.create_function(|_, ()| -> Result<()> { Err(mlua::Error::runtime("boom")) })?;
            for i in 0..3 { println!("err {i} {:?}", e.call::<()>(()).map_err(|e| e.to_string().chars().take(40).collect::<String>())); }
            println!("done");
        }
        "warn_err" => {
            lua.set_warning_function(|_, _, _| Err(mlua::Error::runtime("no")));
            lua.warning("hello", false);
            println!("survived");
        }
        "warn_lua" => {
            lua.set_warning_function(|_, _, _| Err(mlua::Error::runtime("no")));
            println!("{:?}", lua.load("warn('x')").exec().map_err(|e| e.to_string()));
        }
        "drop_panic" => {
            struct P; impl Drop for P { fn drop(&mut self) { panic!("drop panic") } }
            let ud = lua.create_any_userdata(P)?;
            drop(ud);
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lua.gc_collect()));
            println!("gc -> {:?}", r.map_err(|_| "panic"));
            println!("after {:?}", lua.load("return 1+1").eval::<i32>());
        }
        "drop_panic_auto" => {
            struct P; impl Drop for P { fn drop(&mut self) { panic!("drop panic") } }
            lua.globals().set("mk", lua.create_function(|lua, ()| lua.create_any_userdata(P))?)?;
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lua.load("for i=1,1000000 do mk() local t = {} end").exec()));
            println!("loop -> {:?}", r.map_err(|_| "panic"));
            println!("after {:?}", lua.load("return 1+1").eval::<i32>());
        }
        "reset_rust_frame" => {
            lua.globals().set("reset", lua.create_function(|lua, th: Thread| {
                let f = lua.create_function(|_, ()| Ok(1))?;
                println!("reset: {:?}", th.reset(f));
                Ok(())
            })?)?;
            // co1 runs a Rust callback that resumes co2 with Thread::resume, co2 resets co1.
            lua.globals().set("rust_resume", lua.create_function(|_, (th, a, b): (Thread, Value, Value)| {
                println!("in rust_resume a={a:?} b={b:?}");
                let r = th.resume::<()>(());
                println!("resume inner {:?}", r);
                Ok((a, b))
            })?)?;
            let r = lua.load(r#"
                local co1, co2
                co2 = coroutine.create(function() reset(co1) end)
                co1 = coroutine.create(function()
                    return rust_resume(co2, "aaa", "bbb")
                end)
                return coroutine.resume(co1)
            "#).eval::<MultiValue>();
            println!("{:?}", r.map_err(|e| e.to_string()));
        }
        "reset_error_bool" => {
            let f: mlua::Function = lua.load(r#"return function()
                local x <close> = setmetatable({}, {__close = function() error(true) end})
                coroutine.yield(1)
            end"#).eval()?;
            let th = lua.create_thread(f)?;
            th.resume::<i32>(())?;
            let g = lua.create_function(|_, ()| Ok(()))?;
            println!("reset -> {:?}", th.reset(g));
        }
        "big_collect" => {
            // A table with 1.1 million elements converted to Vec<Value>
            let t: Table = lua.load("local t = {} for i=1,1100000 do t[i] = {} end return t").eval()?;
            let r = t.sequence_values::<Value>().collect::<Result<Vec<_>>>();
            println!("vec {:?}", r.map(|v| v.len()));
        }
        "big_collect_str" => {
            let v: Value = lua.load("local t = {} for i=1,600000 do t['k'..i] = 'v'..i end return t").eval()?;
            let r = serde_json::to_string(&v.to_serializable().sort_keys(true));
            println!("sorted {:?}", r.map(|s| s.len()));
        }
        "big_fmt" => {
            let v: Value = lua.load("local t = {} for i=1,600000 do t['k'..i] = i end return t").eval()?;
            println!("fmt {}", format!("{v:#?}").len());
        }
        "big_ret" => {
            println!("{:?}", lua.load("return table.unpack({}, 1, 2000000)").eval::<MultiValue>().map(|m| m.len()).map_err(|e| e.to_string()));
            println!("{:?}", lua.load("local t = {} for i=1,900000 do t[i] = {} end return table.unpack(t)").eval::<MultiValue>().map(|m| m.len()).map_err(|e| e.to_string()));
        }
        "serde_misc" => {
            let v: Value = lua.load("return {}").eval()?;
            println!("empty detect: {:?}", serde_json::to_string(&v.to_serializable().detect_mixed_tables(true)));
            lua.globals().set("arr", lua.array_metatable())?;
            let v: Value = lua.load("return setmetatable({a=5}, arr)").eval()?;
            println!("array mt plain: {:?}", serde_json::to_string(&v.to_serializable()));
            println!("array mt detect: {:?}", serde_json::to_string(&v.to_serializable().detect_mixed_tables(true)));
            let v: Value = lua.load("return setmetatable({}, arr)").eval()?;
            println!("empty array mt detect: {:?}", serde_json::to_string(&v.to_serializable().detect_mixed_tables(true)));

            #[derive(serde::Deserialize, Debug)] struct S { #[allow(dead_code)] a: u32 }
            let v: Value = lua.load("return {a = 10/2}").eval()?;
            println!("u32 from 5.0: {:?}", lua.from_value::<S>(v).map_err(|e| e.to_string()));
            let v: Value = lua.load("return 10/2").eval()?;
            println!("Value of 10/2: {:?}; as u32 via FromLua: {:?}", v, lua.unpack::<u32>(v.clone()));
            let v: Value = lua.load("return {1, print, 3}").eval()?;
            println!("skip: {:?}", serde_json::to_string(&v.to_serializable().deny_unsupported_types(false)));
            let v: Value = lua.load("return {1, nil, 3}").eval()?;
            println!("hole: {:?}", serde_json::to_string(&v.to_serializable()));
            println!("hole detect: {:?}", serde_json::to_string(&v.to_serializable().detect_mixed_tables(true)));
            let v: Value = lua.load("return {[1]='a',[3]='c'}").eval()?;
            println!("border from_value Vec: {:?}", lua.from_value::<Vec<Option<String>>>(v).map_err(|e| e.to_string()));
            let v: Value = lua.load("local t = {} t.self = t return t").eval()?;
            println!("rec: {:?}", serde_json::to_string(&v.to_serializable()).map_err(|e| e.to_string()));
            println!("rec allowed: {:?}", serde_json::to_string(&v.to_serializable().deny_recursive_tables(false)).map_err(|e| e.to_string()));
            let v: Value = lua.load("return {[true]=1}").eval()?;
            println!("bool key: {:?}", serde_json::to_string(&v.to_serializable()).map_err(|e| e.to_string()));
            let v: Value = lua.load("return math.huge").eval()?;
            println!("inf: {:?}", serde_json::to_string(&v.to_serializable()).map_err(|e| e.to_string()));
            let map: std::collections::BTreeMap<Option<i32>, i32> = [(Some(1), 1)].into_iter().collect();
            println!("opt key ok {:?}", lua.to_value(&map).map(|_| ()));
            // value printing and conversions
            println!("to_string 1.0 = {:?}, 1e100 = {:?}", Value::Number(1.0).to_string(), Value::Number(1e15).to_string());
            println!("eq 2^53+1 vs 2^53 float: {}", Value::Integer((1i64<<53)+1) == Value::Number((1i64<<53) as f64));
            println!("lua says: {:?}", lua.load("return (1<<53)+1 == 2.0^53").eval::<bool>());
        }
        "convert_misc" => {
            println!("i32 from 1.5: {:?}", lua.unpack::<i32>(Value::Number(1.5)));
            println!("u8 from 300: {:?}", lua.unpack::<u8>(Value::Integer(300)).map_err(|e| e.to_string()));
            println!("i32 from '1.5': {:?}", lua.load("return '1.5'").eval::<i32>().map_err(|e| e.to_string()));
            println!("f32 from huge: {:?}", lua.unpack::<f32>(Value::Number(1e300)));
            println!("u64 max into lua: {:?}", lua.pack(u64::MAX));
            println!("bool from 0: {:?}", lua.unpack::<bool>(Value::Integer(0)));
            println!("char: {:?}", lua.unpack::<char>(Value::Integer(65)));
            println!("cstring nul: {:?}", lua.load("return 'a\\0b'").eval::<std::ffi::CString>().map_err(|e| e.to_string()));
            println!("array [i32;2] from 3: {:?}", lua.load("return {1,2,3}").eval::<[i32;2]>().map_err(|e| e.to_string()));
            println!("hashmap bad: {:?}", lua.load("return {a=1,b='x'}").eval::<std::collections::HashMap<String,i32>>().map_err(|e| e.to_string()));
            let t: Table = lua.load("return {1,2,3}").eval()?;
            let mut it = t.sequence_values::<String>();
            println!("seq coerces: {:?}", it.next());
            drop(it);
            println!("raw_remove float 1.0 on {{1,2,3}}: {:?} len {}", t.raw_remove(1.0), t.raw_len());
            let t: Table = lua.load("return {1,2,3}").eval()?;
            println!("contains nil key? {:?}", t.contains_key(Value::Nil));
            println!("get nil key? {:?}", t.get::<Value>(Value::Nil));
            println!("raw_get nan? {:?}", t.raw_get::<Value>(f64::NAN));
        }
        "fn_misc" => {
            let f: mlua::Function = lua.load("return function(...) return select('#', ...), ... end").eval()?;
            let b = f.bind((1, Value::Nil))?;
            println!("bind nil: {:?}", b.call::<MultiValue>((Value::Nil, 2)));
            let t: Vec<i32> = (0..300).collect();
            println!("bind too many: {:?}", f.bind(mlua::Variadic::from(t)).map(|_| ()).map_err(|e| e.to_string()));
            let g: mlua::Function = lua.load("return function() return x end").eval()?;
            println!("env: {:?}", g.environment().map(|t| t == lua.globals()));
            let e = lua.create_table()?; e.set("x", 7)?;
            println!("set env: {:?} -> {:?}", g.set_environment(e), g.call::<i32>(()));
            let h: mlua::Function = lua.load("return function() return 1 end").eval()?;
            println!("no env upvalue: {:?} {:?}", h.environment().is_none(), h.set_environment(lua.create_table()?));
            let stripped = lua.load(&g.dump(true)[..]).into_function()?;
            println!("stripped env: {:?}", stripped.environment().is_some());
            println!("info: {:?}", g.info());
            println!("dump of rust fn: {}", lua.create_function(|_, ()| Ok(()))?.dump(false).len());
            println!("{}", std::mem::size_of::<AnyUserData>());
        }
        "path_harness" => {
            let t: Table = lua.load("return {a={b={1,2,{c=3}}}, ['x y']=1}").eval()?;
            use mlua::ObjectLike;
            let alpha: Vec<char> = "ab.[]?'\"\\-1 é".chars().collect();
            let mut count = 0u64;
            let mut buf = vec![0usize; 0];
            for len in 0..=6usize {
                buf = vec![0; len];
                loop {
                    let s: String = buf.iter().map(|&i| alpha[i]).collect();
                    let _ = t.get_path::<Value>(&s);
                    count += 1;
                    let mut i = 0;
                    while i < len { buf[i] += 1; if buf[i] == alpha.len() { buf[i] = 0; i += 1; } else { break; } }
                    if i == len { break; }
                }
            }
            let _ = buf;
            println!("path ok {count}");
            println!("{:?}", t.get_path::<i32>("a.b[3].c"));
            println!("{:?}", t.get_path::<Value>("['x y']"));
            println!("{:?}", t.get_path::<Value>(r#"a["b"]"#).map(|v| v.type_name()));
            println!("{:?}", t.get_path::<Value>("a.b[-9223372036854775808]").map_err(|e| e.to_string()));
            println!("{:?}", t.get_path::<Value>("q?.r").map_err(|e| e.to_string()));
            println!("{:?}", t.get_path::<Value>("[1]").map_err(|e| e.to_string()));
        }
        "gc_after_panic" => {
            struct P; impl Drop for P { fn drop(&mut self) { panic!("drop panic") } }
            println!("gc running before: {}", lua.gc_is_running());
            let ud = lua.create_any_userdata(P)?;
            drop(ud);
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lua.gc_collect()));
            println!("gc -> {:?}", r.map_err(|_| "panic"));
            println!("gc running after: {}", lua.gc_is_running());
            let before = lua.used_memory();
            lua.load("for i=1,2000000 do local t = {} end").exec()?;
            println!("memory before loop {before}, after loop {}", lua.used_memory());
            println!("gc_collect again {:?}", lua.gc_collect().map_err(|e| e.to_string()));
            println!("memory after collect {}", lua.used_memory());
            println!("traceback: {:?}", lua.load("error('x')").exec().map_err(|e| e.to_string()));
            std::mem::forget(lua);
        }
        "many_ret" => {
            for n in [5, 10, 15, 18, 20, 25, 30, 40, 100, 1000, 100000] {
                let f: mlua::Function = lua.load("local n = ...; local t = {} for i=1,n do t[i] = 'v'..i end return table.unpack(t)").into_function()?;
                let r = f.call::<MultiValue>(n);
                println!("{n}: {:?}", r.map(|m| m.len()).map_err(|e| e.to_string()));
            }
        }
        "many_args_cb" => {
            let f = lua.create_function(|_, a: MultiValue| Ok(a.len()))?;
            lua.globals().set("f", f)?;
            for n in [5, 20, 30, 100, 1000, 100000] {
                let g: mlua::Function = lua.load("local n = ...; local t = {} for i=1,n do t[i] = 'v'..i end return f(table.unpack(t))").into_function()?;
                println!("{n}: {:?}", g.call::<usize>(n).map_err(|e| e.to_string()));
            }
        }
        "many_resume" => {
            let f: mlua::Function = lua.load("return function(n) local t = {} for i=1,n do t[i] = 'v'..i end return table.unpack(t) end").eval()?;
            for n in [5, 30, 100, 100000] {
                let th = lua.create_thread(f.clone())?;
                println!("{n}: {:?}", th.resume::<MultiValue>(n).map(|m| m.len()).map_err(|e| e.to_string()));
            }
        }
        "longjmp_state" => {
            let t = lua.create_table()?;
            lua.globals().set("f", lua.create_function(move |_, ()| { t.set(Value::Nil, 1)?; Ok(()) })?)?;
            println!("state before: {:?}", lua.current_thread().to_pointer());
            let r = lua.load(r#"
                local co = coroutine.create(function() f() end)
                print(coroutine.resume(co))
                co = nil
                collectgarbage() collectgarbage()
                local junk = {} for i=1,1000 do junk[i] = coroutine.create(print) end
            "#).exec();
            println!("exec {:?}", r.map_err(|e| e.to_string()));
            println!("state after: {:?}", lua.current_thread().to_pointer());
            println!("still main thread? {:?}", lua.current_thread().status());
            let s = lua.create_string("after");
            println!("{:?}", s.map(|s| s.to_string_lossy()));
            lua.gc_collect()?;
            println!("t {:?}", lua.create_table().map(|t| t.raw_len()));
            println!("load {:?}", lua.load("return 1+1").eval::<i32>().map_err(|e| e.to_string()));
        }
        "pool_loss" => {
            // Each longjmp over a callback frame leaks the reserved WrappedFailure slot count.
            let t = lua.create_table()?;
            let f = lua.create_function(move |_, ()| { t.set(Value::Nil, 1)?; Ok(()) })?;
            lua.globals().set("f", f)?;
            lua.load("for i=1,5 do pcall(f) end").exec()?;
            println!("ok");
        }
        "alloc_fail" => {
            println!("{:?}", lua.load("return #string.rep('x', 1<<30)").eval::<i64>().map_err(|e| e.to_string()));
            println!("{:?}", lua.load("return pcall(string.rep, 'x', (1<<31)-2)").eval::<MultiValue>().map_err(|e| e.to_string()));
            println!("survived");
        }
        "cap_huge" => {
            println!("{:?}", lua.create_table_with_capacity(usize::MAX, 0).map(|_| ()).map_err(|e| e.to_string()));
            println!("survived");
        }
        "shebang" => {
            let dir = std::env::var("TMPDIR").unwrap_or("/tmp".into());
            let p = std::path::PathBuf::from(dir).join("sb.lua");
            std::fs::write(&p, "#!/usr/bin/env lua\nreturn 42\n").unwrap();
            println!("via mlua load(path): {:?}", lua.load(p.as_path()).eval::<i32>().map_err(|e| e.to_string()));
            println!("via dofile: {:?}", lua.load(format!("return dofile('{}')", p.display())).eval::<i32>().map_err(|e| e.to_string()));
            std::fs::write(&p, b"\xef\xbb\xbfreturn 43\n").unwrap();
            println!("BOM via mlua load(path): {:?}", lua.load(p.as_path()).eval::<i32>().map_err(|e| e.to_string()));
            println!("BOM via dofile: {:?}", lua.load(format!("return dofile('{}')", p.display())).eval::<i32>().map_err(|e| e.to_string()));
        }
        "ud_eq" => {
            struct A; struct B;
            lua.register_userdata_type::<A>(|r| { r.add_meta_function("__eq", |_, ()| Ok(true)); })?;
            lua.register_userdata_type::<B>(|_| {})?;
            let a = lua.create_any_userdata(A)?; let b = lua.create_any_userdata(B)?;
            lua.globals().set("a", &a)?; lua.globals().set("b", &b)?;
            println!("lua a==b: {:?}", lua.load("return a == b").eval::<bool>());
            println!("Value::equals a,b: {:?}", Value::UserData(a.clone()).equals(&Value::UserData(b.clone())));
            println!("Value::equals b,a: {:?}", Value::UserData(b).equals(&Value::UserData(a)));
            let t1: Table = lua.load("return setmetatable({}, {__eq = setmetatable({}, {__call = function() return true end})})").eval()?;
            let t2 = lua.create_table()?;
            lua.globals().set("t1", &t1)?; lua.globals().set("t2", &t2)?;
            println!("lua t1==t2: {:?}", lua.load("return t1 == t2").eval::<bool>());
            println!("Table::equals: {:?}", t1.equals(&t2).map_err(|e| e.to_string()));
        }
        "handle_after_drop" => {
            let s = lua.create_string("x")?;
            let t = lua.create_table()?;
            drop(lua);
            println!("table dbg: {:?}", std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| format!("{:?}", t))).map_err(|_| "panic"));
            println!("string dbg: {:?}", std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| format!("{:?}", s))).map_err(|_| "panic"));
        }
        "gc_inc_in_gc" => {
            lua.globals().set("g", lua.create_function(|lua, ()| { lua.gc_inc(0,0,0); Ok(()) })?)?;
            lua.set_warning_function(|_, m, _| { println!("warn: {m}"); Ok(()) });
            lua.load("warn('@on'); setmetatable({}, {__gc = function() g() end}); collectgarbage()").exec()?;
            println!("done");
        }
        "hook_big" => {
            println!("{:?}", lua.set_hook(mlua::HookTriggers::new().every_nth_instruction(u32::MAX), |_, _| Ok(mlua::VmState::Continue)));
            println!("{:?}", lua.load("return 1").eval::<i32>());
        }
        "tostring_num" => {
            println!("{:?} vs lua {:?}", Value::Number(1.0).to_string(), lua.load("return tostring(1.0)").eval::<String>());
            println!("{:?} vs lua {:?}", Value::Number(1e100).to_string().map(|s| s.len()), lua.load("return tostring(1e100)").eval::<String>());
        }
        "bytes_bad" => {
            println!("{:?}", lua.load(&b"\x1bLua"[..]).exec().map_err(|e| e.to_string()));
            let f: Value = lua.load("return load(string.dump(function() return 7 end))").eval()?;
            println!("script can load bytecode in safe mode: {:?}", f.type_name());
        }
        "bytecode_crash" => {
            // A plain script in Lua::new(): no Rust involvement beyond running it.
            let r = lua.load(r#"
                local d = string.dump(function(a) local t = {1,2,3} return t[a], #t, a end)
                local seed = 12345
                local function rnd(n) seed = (seed * 1103515245 + 12345) & 0x7fffffff return seed % n + 1 end
                for i = 1, 200000 do
                    local p = rnd(#d)
                    local s = d:sub(1, p-1) .. string.char(rnd(256)-1) .. d:sub(p+1)
                    local f = load(s, "x", "b")
                    if f then pcall(f, 1) end
                end
                return "no crash"
            "#).eval::<String>();
            println!("{:?}", r.map_err(|e| e.to_string()));
        }
        "drop_resumes" => {
            use std::future::Future;
            let f: mlua::Function = lua.load(r#"
                coroutine.yield()
                side_effect = "script kept running after the future was dropped"
            "#).into_function()?;
            {
                let mut fut = Box::pin(f.call_async::<()>(()));
                let mut cx = std::task::Context::from_waker(futures_util::task::noop_waker_ref());
                println!("poll: {:?}", fut.as_mut().poll(&mut cx).is_pending());
                println!("before drop: {:?}", lua.globals().get::<Value>("side_effect")?);
            }
            println!("after drop: {:?}", lua.globals().get::<Value>("side_effect")?);
            let f: mlua::Function = lua.load(r#"
                local x = coroutine.yield()
                error("error raised during drop")
            "#).into_function()?;
            {
                let mut fut = Box::pin(f.call_async::<()>(()));
                let mut cx = std::task::Context::from_waker(futures_util::task::noop_waker_ref());
                let _ = fut.as_mut().poll(&mut cx);
            }
            println!("second drop ok");
        }
        "async_no_coroutine" => {
            lua.globals().set("coroutine", Value::Nil)?;
            println!("{:?}", lua.create_async_function(|_, ()| async { Ok(()) }).map(|_| ()).map_err(|e| e.to_string()));
        }
        "cross_mt" => {
            let b = Lua::new();
            let s_a = lua.create_string("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")?;
            let t_b = b.create_table()?;
            let t_a = lua.create_table()?;
            println!("indexes share a slot: same position in each ref stack; a string in A, a table in B");
            let r = t_a.set_metatable(Some(t_b));
            println!("set_metatable with a handle from another Lua: {:?}", r);
            println!("A's table now has metatable? {}", t_a.has_metatable());
            let v = t_a.get::<Value>("missing");
            println!("get: {:?}", v.map(|v| v.type_name()).map_err(|e| e.to_string()));
            drop(s_a);
            lua.gc_collect()?; lua.gc_collect()?;
            println!("after gc: {:?}", t_a.get::<Value>("missing").map(|v| v.type_name()).map_err(|e| e.to_string()));
        }
        "cross_thread" => {
            let b = Lua::new();
            let f_a = lua.create_function(|_, ()| Ok("function from lua A ran"))?;
            let f_b = b.create_function(|_, ()| Ok("function from lua B ran"))?;
            let th = lua.create_thread(f_b)?;
            println!("create_thread with B's function: {:?}", th.resume::<String>(()));
            drop(f_a);
        }
        "scoped_method_uaf" => {
            struct Data(String);
            lua.scope(|scope| {
                let ud = scope.create_any_userdata(Data(String::from("secret-secret-secret-secret")), |reg| {
                    reg.add_method("get", |_, this, ()| Ok(this.0.clone()));
                })?;
                lua.globals().set("ud", ud)?;
                lua.load("m = ud.get; print('inside scope', m(ud))").exec()?;
                Ok(())
            })?;
            // Scope ended, the Rust value has been dropped.
            let junk: Vec<String> = (0..1000).map(|i| format!("junk-junk-junk-junk-junk-{i}")).collect();
            let r = lua.load("return m(ud)").eval::<LuaString>();
            println!("after scope: {:?}", r.map(|s| s.to_string_lossy()).map_err(|e| e.to_string()));
            drop(junk);
        }
        "scoped_method_reuse" => {
            struct Data(u64);
            struct Other(String);
            lua.scope(|scope| {
                let ud = scope.create_any_userdata(Data(7), |reg| {
                    reg.add_method("get", |_, this, ()| Ok(this.0));
                })?;
                lua.globals().set("ud", ud)?;
                lua.load("m = ud.get; ud = nil").exec()?;
                Ok(())
            })?;
            lua.gc_collect()?; lua.gc_collect()?;
            // New userdata blocks of the same size can land on the old address.
            let mut keep = vec![];
            for i in 0..2000 {
                let o = lua.create_any_userdata(Other(format!("x{i}")))?;
                lua.globals().set("o", &o)?;
                keep.push(o);
                if let Ok(v) = lua.load("return m(o)").eval::<i64>() { println!("method accepted foreign userdata at try {i}, read {v:#x}"); return Ok(()); }
            }
            println!("no reuse hit");
        }
        "destroy_in_method" => {
            struct Data(String);
            lua.register_userdata_type::<Data>(|reg| {
                reg.add_method("boom", |_, this, ud: AnyUserData| {
                    let r = ud.borrow::<Data>()?;
                    ud.destroy()?;
                    drop(r);
                    // The value behind `this` has been freed at this point.
                    let junk: Vec<String> = (0..100).map(|_| "Z".repeat(32)).collect();
                    let out = this.0.clone();
                    drop(junk);
                    Ok(out)
                });
            })?;
            let ud = lua.create_any_userdata(Data("live-live-live-live-live-live-live".repeat(1)))?;
            lua.globals().set("ud", ud)?;
            let r = lua.load("return ud:boom(ud)").eval::<LuaString>();
            println!("{:?}", r.map(|s| s.to_string_lossy()).map_err(|e| e.to_string()));
        }
        "getters_only" => {
            struct G; struct M; struct GM;
            lua.register_userdata_type::<G>(|r| { r.add_field_method_get("a", |_, _| Ok(1)); })?;
            lua.register_userdata_type::<M>(|r| { r.add_method("a", |_, _, ()| Ok(1)); })?;
            lua.register_userdata_type::<GM>(|r| { r.add_field_method_get("a", |_, _| Ok(1)); r.add_method("b", |_, _, ()| Ok(1)); })?;
            lua.globals().set("g", lua.create_any_userdata(G)?)?;
            lua.globals().set("m", lua.create_any_userdata(M)?)?;
            lua.globals().set("gm", lua.create_any_userdata(GM)?)?;
            for code in ["return g.zz", "return m.zz", "return gm.zz", "return g[{}]", "return g[true]", "g.zz = 1", "g[1] = 1", "g[true] = 1"] {
                println!("{code}: {:?}", lua.load(code).eval::<Value>().map(|v| v.type_name()).map_err(|e| e.to_string().lines().next().unwrap().to_string()));
            }
        }
        "hook_yield_gc" => {
            let f = lua.load(r#"
                local a = {1,2,3}
                local s = "hello" .. tostring(os.time())
                local n = 0
                for i = 1, 200 do n = n + #a + #s end
                return n
            "#).into_function()?;
            let co = lua.create_thread(f)?;
            co.set_hook(mlua::HookTriggers::new().every_nth_instruction(7), |_, _| Ok(mlua::VmState::Yield))?;
            let mut yields = 0;
            loop {
                let r = co.resume::<Value>(());
                match r {
                    Ok(v) => {
                        if co.status() != mlua::ThreadStatus::Resumable { println!("finished: {v:?} after {yields} yields"); break; }
                        yields += 1;
                        lua.gc_collect()?; // the host runs a GC while the coroutine is suspended
                        let _junk = lua.create_table()?;
                    }
                    Err(e) => { println!("error after {yields} yields: {}", e.to_string().lines().next().unwrap()); break; }
                }
            }
        }
        "map_nil_key" => {
            let map: std::collections::BTreeMap<Option<i32>, i32> = [(None, 1)].into_iter().collect();
            let opts = mlua::SerializeOptions::new().serialize_none_to_null(false);
            println!("{:?}", lua.to_value_with(&map, opts).map(|_| ()));
        }
        _ => panic!("unknown"),
    }
    Ok(())
}
