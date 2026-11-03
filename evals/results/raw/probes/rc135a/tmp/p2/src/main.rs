use mlua::prelude::*;
use mlua::{Function, Nil, Table, Thread, UserData, Value};
use std::collections::HashMap;

struct A(u64);
struct B<const N: usize>(String);

fn try_mt_confusion(k: usize) -> LuaResult<bool> {
    let lua = Lua::new();
    lua.register_userdata_type::<A>(|r| { r.add_field_method_get("v", |_, t| Ok(t.0)); })?;
    {
        let ud = lua.create_any_userdata(A(0x4141414141414141))?;
        let _ = ud.borrow::<A>()?; // populates last_checked_userdata_mt
    }
    lua.register_userdata_type::<A>(|_| {})?;
    lua.gc_collect()?; lua.gc_collect()?;
    let hold: Vec<Table> = (0..k).map(|_| lua.create_table().unwrap()).collect();
    let ud = lua.create_any_userdata(B::<7>("not an A".to_string()))?;
    let is_a = ud.is::<A>();
    if is_a {
        let b = ud.borrow::<A>()?;
        println!("k={k}: B userdata accepted as A; A.0 = {:#x}", b.0);
    }
    drop(hold);
    Ok(is_a)
}

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "mt_cache" => {
            let mut hits = 0;
            for k in 0..64 { if try_mt_confusion(k)? { hits += 1; } }
            println!("hits {hits}/64");
        }
        "warning_err" => {
            lua.set_warning_function(|_, _, _| Err(LuaError::runtime("warning error")));
            println!("calling warning");
            lua.warning("hi", false);
            println!("survived");
        }
        "warning_panic" => {
            lua.set_warning_function(|_, _, _| panic!("in warn"));
            println!("calling warning");
            lua.warning("hi", false);
            println!("survived");
        }
        "cross_mt_release" => {
            let lua2 = Lua::new();
            let s = lua.create_string("not a table, a string")?;
            let a = lua.create_table()?;
            let _ = &s;
            let mts: Vec<Table> = (0..4).map(|_| lua2.create_table().unwrap()).collect();
            for (i, mt) in mts.iter().enumerate() {
                println!("try {i}");
                a.set_metatable(Some(mt.clone()))?;
                println!(" has_metatable={}", a.has_metatable());
                println!(" a.metatable raw_len={:?}", a.metatable().map(|m| m.raw_len()));
                println!(" index: {:?}", a.get::<Value>("zzz").map_err(|e| e.to_string()));
            }
        }
        "cross_fn_exec" => {
            let lua2 = Lua::new();
            let f1: Function = lua.load("return function() return 'function from lua1' end").eval()?;
            let f2: Function = lua2.load("return function() return 'function from lua2' end").eval()?;
            let th = lua.create_thread(f2.clone())?;
            println!("thread on lua1 with f2 ran: {:?}", th.resume::<String>(()).map_err(|e| e.to_string()));
            let _ = f1;
        }
        "close_err_bool" => {
            let th: Thread = lua.load(r#"
                coroutine.create(function()
                    local x <close> = setmetatable({}, {__close = function() error(true) end})
                    coroutine.yield()
                end)
            "#).eval()?;
            th.resume::<()>(())?;
            let f = lua.create_function(|_, ()| Ok(()))?;
            println!("reset: {:?}", th.reset(f).map_err(|e| e.to_string()));
            let th: Thread = lua.load(r#"
                coroutine.create(function()
                    local x <close> = setmetatable({}, {__close = function() error(false) end})
                    coroutine.yield()
                end)
            "#).eval()?;
            th.resume::<()>(())?;
            let f = lua.create_function(|_, ()| Ok(()))?;
            println!("reset(false): {:?}", th.reset(f).map_err(|e| e.to_string()));
        }
        "gc_in_finalizer" => {
            let f = lua.create_function(|lua, ()| {
                println!("gc_is_running in __gc: {}", lua.gc_is_running());
                println!("calling gc_inc");
                let m = lua.gc_inc(0, 0, 0);
                println!("gc_inc -> {:?}", m);
                Ok(())
            })?;
            lua.globals().set("rf", f)?;
            lua.set_warning_function(|_, m, _| { println!("WARN: {m}"); Ok(()) });
            lua.load("setmetatable({}, {__gc = rf})").exec()?;
            lua.gc_collect()?; lua.gc_collect()?;
            println!("after");
        }
        "panic_gc_dead" => {
            struct PD; impl UserData for PD {}
            impl Drop for PD { fn drop(&mut self) { if std::env::var("X").is_err() { panic!("drop panic"); } } }
            lua.globals().set("x", PD)?;
            lua.load("x = nil").exec()?;
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lua.gc_collect()));
            println!("gc_collect panicked: {}", r.is_err());
            let before = lua.used_memory();
            println!("collect after: {:?}", lua.gc_collect());
            let f: LuaResult<i64> = lua.load("collectgarbage('collect'); local c = collectgarbage('count'); for i=1,2e6 do local t = {} end; return collectgarbage('count') - c").eval();
            println!("kb grown while allocating garbage: {:?} (used before {before})", f.map_err(|e| e.to_string()));
            println!("Function call traceback: {:?}", lua.load("error('x')").exec().map_err(|e| e.to_string()));
            println!("end");
        }
        "pairs_error_key" => {
            let f = lua.create_function(|_, ()| -> LuaResult<()> { Err(LuaError::runtime("boom")) })?;
            lua.globals().set("fail", f)?;
            let t: Table = lua.load(r#"
                local ok, e = pcall(fail)
                local t = {[e] = 1, x = 2, y = 3}
                return t
            "#).eval()?;
            println!("iterating");
            let m: LuaResult<HashMap<String, Value>> = t.pairs::<Value, Value>().map(|kv| kv.map(|(k, v)| (format!("{k:?}"), v))).collect();
            println!("{:?}", m.map(|m| m.len()));
        }
        "pairs_error_key_serde" => {
            let f = lua.create_function(|_, ()| -> LuaResult<()> { Err(LuaError::runtime("boom")) })?;
            lua.globals().set("fail", f)?;
            let t: Value = lua.load(r#"
                local ok, e = pcall(fail)
                return {[e] = 1, x = 2, y = 3}
            "#).eval()?;
            println!("{:?}", lua.from_value::<HashMap<String, i32>>(t).map_err(|e| e.to_string()));
        }
        "deep_serde" => {
            let t: Value = lua.load("local t = {} for i=1,200000 do t = {t} end return t").eval()?;
            println!("built");
            println!("{:?}", serde_json::to_string(&t).map(|s| s.len()).map_err(|e| e.to_string()));
        }
        "deep_de" => {
            let t: Value = lua.load("local t = {} for i=1,200000 do t = {t} end return t").eval()?;
            println!("built");
            println!("{:?}", lua.from_value::<serde_json::Value>(t).map(|_| "ok").map_err(|e| e.to_string()));
        }
        "deep_debug" => {
            let t: Value = lua.load("local t = {} for i=1,200000 do t = {t} end return t").eval()?;
            println!("built");
            println!("{}", format!("{t:#?}").len());
        }
        "ser_nil_key" => {
            let mut m: HashMap<Option<i32>, i32> = HashMap::new();
            m.insert(None, 1);
            println!("{:?}", lua.to_value_with(&m, mlua::SerializeOptions::new().serialize_none_to_null(false)).map(|_| "ok").map_err(|e| e.to_string()));
        }
        "table_from_nil" => {
            println!("{:?}", lua.create_table_from(vec![(Nil, 1)]).map(|_| "ok").map_err(|e| e.to_string()));
        }
        "raw_remove_nil" => {
            let t = lua.create_table()?;
            println!("{:?}", t.raw_remove(Nil).map_err(|e| e.to_string()));
        }
        "is_sync_copy" => {
            use std::cell::Cell; use std::marker::PhantomData;
            struct IsSync<'a, T> { is_sync: &'a Cell<bool>, _marker: PhantomData<T> }
            impl<T> Clone for IsSync<'_, T> { fn clone(&self) -> Self { self.is_sync.set(false); IsSync { is_sync: self.is_sync, _marker: PhantomData } } }
            impl<T: Sync> Copy for IsSync<'_, T> {}
            fn is_sync<T>() -> bool { let is_sync = Cell::new(true); let _ = [IsSync::<T> { is_sync: &is_sync, _marker: PhantomData }].clone(); is_sync.get() }
            println!("is_sync::<String>() = {}, is_sync::<u32>() = {}, is_sync::<Cell<u32>>() = {}", is_sync::<String>(), is_sync::<u32>(), is_sync::<Cell<u32>>());
        }
        "nil_key_co" => {
            let f = lua.create_function(|_, (t, k): (Table, Value)| { t.set(k, 1)?; Ok(()) })?;
            lua.globals().set("rust_set", f)?;
            let main = lua.current_thread().to_pointer();
            lua.load("coroutine.wrap(function() print('pcall in co:', pcall(rust_set, {}, nil)) end)()").exec()?;
            let now = lua.current_thread();
            println!("main thread {:?}, current_thread now {:?}, status {:?}", main, now.to_pointer(), now.status());
            drop(now);
            lua.load("collectgarbage(); collectgarbage()").exec().map_err(|e| println!("exec err: {e}")).ok();
            lua.gc_collect()?; lua.gc_collect()?;
            let v: Vec<Vec<u8>> = (0..2000).map(|_| vec![0xAAu8; 1000]).collect();
            println!("using state after coroutine collected");
            println!("{:?}", lua.create_table().map(|_| "ok").map_err(|e| e.to_string()));
            println!("{:?}", lua.load("return 1+1").eval::<i32>().map_err(|e| e.to_string()));
            drop(v);
        }
        "rep_huge" => {
            println!("{:?}", lua.load("return pcall(string.rep, 'x', 1 << 42)").eval::<LuaMultiValue>().map_err(|e| e.to_string()));
            println!("survived");
        }
        "rep_huge_limit" => {
            lua.set_memory_limit(1 << 30)?;
            println!("{:?}", lua.load("return pcall(string.rep, 'x', 1 << 42)").eval::<LuaMultiValue>().map_err(|e| e.to_string()));
            println!("survived");
        }
        "pairs_stop2" => {
            let t: Table = lua.load("local t = {} for i=1,50 do t['k'..i] = i end return t").eval()?;
            let total = t.pairs::<String, i32>().count();
            // Mark the key that iteration reaches first as unconvertible
            let first: String = t.pairs::<String, i32>().next().unwrap()?.0;
            t.set(first.as_str(), lua.create_table()?)?;
            let (mut ok, mut err) = (0, 0);
            for kv in t.pairs::<String, i32>() { match kv { Ok(_) => ok += 1, Err(_) => err += 1 } }
            println!("total={total} ok={ok} err={err}");
        }
        "pcall_yield_default" => {
            let r = lua.load(r#"
              local co = coroutine.wrap(function() return pcall(function() coroutine.yield(1); return 2 end) end)
              return co(), co()
            "#).eval::<LuaMultiValue>();
            println!("{:?}", r.map_err(|e| e.to_string()));
        }
        "rep_gb" => {
            println!("{:?}", lua.load("return pcall(string.rep, 'x', 1 << 30)").eval::<LuaMultiValue>().map(|v| v.len()).map_err(|e| e.to_string()));
            println!("survived");
        }
        "mt_cache2" => {
            use mlua::ffi;
            lua.register_userdata_type::<A>(|r| { r.add_field_method_get("v", |_, t| Ok(t.0)); })?;
            let ud = lua.create_any_userdata(A(0x4141414141414141))?;
            let _ = ud.borrow::<A>()?;
            let mt_ptr: i64 = unsafe { lua.exec_raw::<i64>(&ud, |st| {
                ffi::lua_getmetatable(st, -1);
                let p = ffi::lua_topointer(st, -1) as i64;
                ffi::lua_settop(st, 0);
                ffi::lua_pushinteger(st, p);
            }) }?;
            drop(ud);
            lua.register_userdata_type::<A>(|_| {})?;
            lua.gc_collect()?; lua.gc_collect()?;
            let mut hold = Vec::new();
            let mut found = None;
            for i in 0..2000 {
                let t = lua.create_table()?;
                if t.to_pointer() as i64 == mt_ptr { found = Some((i, t)); break; }
                hold.push(t);
            }
            println!("old metatable address reused by plain table: {:?}", found.as_ref().map(|f| f.0));
            drop(found);
            lua.gc_collect()?; lua.gc_collect()?;
            struct Big([u64; 32]);
            let ud = lua.create_any_userdata(Big([0x4242424242424242; 32]))?;
            println!("Big.is::<A>() = {}", ud.is::<A>());
            if let Ok(a) = ud.borrow::<A>() { println!("Big borrowed as A: A.0 = {:#x}", a.0); }
            println!("Big.is::<Big>() = {}", ud.is::<Big>());
        }
        "hashset_int" => {
            use std::collections::{BTreeSet, HashSet};
            let s: HashSet<i32> = [1, 2, 3].into_iter().collect();
            let v = lua.pack(s.clone())?;
            println!("hashset roundtrip: {:?}", lua.unpack::<HashSet<i32>>(v).map_err(|e| e.to_string()));
            let s: BTreeSet<i32> = [1, 2, 3].into_iter().collect();
            let v = lua.pack(s.clone())?;
            println!("btreeset roundtrip: {:?}", lua.unpack::<BTreeSet<i32>>(v).map_err(|e| e.to_string()));
            let s: HashSet<i32> = [5, 7].into_iter().collect();
            let v = lua.pack(s.clone())?;
            println!("no-1 set roundtrip: {:?}", lua.unpack::<HashSet<i32>>(v).map_err(|e| e.to_string()));
            let v: Value = lua.load("{1, 2, a=true}").eval()?;
            println!("mixed set: {:?}", lua.unpack::<HashSet<String>>(v).map_err(|e| e.to_string()));
        }
        "pairs_stop" => {
            let t: Table = lua.load("local t = {} for i=1,5 do t['k'..i] = i end t['k3'] = {} return t").eval()?;
            let mut ok = 0; let mut err = 0;
            for kv in t.pairs::<String, i32>() { match kv { Ok(_) => ok += 1, Err(_) => err += 1 } }
            println!("pairs: ok={ok} err={err} (5 entries, 1 unconvertible)");
            let t: Table = lua.load("{1,2,{},4,5}").eval()?;
            let mut ok = 0; let mut err = 0;
            for v in t.sequence_values::<i32>() { match v { Ok(_) => ok += 1, Err(_) => err += 1 } }
            println!("sequence: ok={ok} err={err}");
        }
        "rep_oom" => {
            println!("{:?}", lua.load("return #string.rep('x', 1 << 45)").eval::<i64>().map_err(|e| e.to_string()));
        }
        "rep_oom_pcall" => {
            println!("{:?}", lua.load("return pcall(string.rep, 'x', 1 << 45)").eval::<LuaMultiValue>().map_err(|e| e.to_string()));
        }
        "value_eq" => {
            let a = Value::Integer(9007199254740993);
            let b = Value::Number(9007199254740992.0);
            println!("Value eq: {}", a == b);
            lua.globals().set("a", a)?; lua.globals().set("b", b)?;
            println!("Lua eq: {}", lua.load("return a == b").eval::<bool>()?);
        }
        "err_key_skip" => {
            let f = lua.create_function(|_, ()| -> LuaResult<()> { Err(LuaError::runtime("boom")) })?;
            lua.globals().set("fail", f)?;
            let t: Value = lua.load("local ok, e = pcall(fail) return {[e] = 1, x = 2, y = 3}").eval()?;
            let o = mlua::DeserializeOptions::new().deny_unsupported_types(false);
            println!("{:?}", lua.from_value_with::<HashMap<String, i32>>(t, o).map_err(|e| e.to_string()));
        }
        "err_key_debug" => {
            let f = lua.create_function(|_, ()| -> LuaResult<()> { Err(LuaError::runtime("boom")) })?;
            lua.globals().set("fail", f)?;
            let t: Value = lua.load("local ok, e = pcall(fail) return {[e] = 1, x = 2, y = 3}").eval()?;
            println!("{t:#?}");
        }
        "panic_then_err" => {
            struct PD; impl UserData for PD {}
            impl Drop for PD { fn drop(&mut self) { panic!("drop panic"); } }
            lua.globals().set("x", PD)?;
            lua.load("x = nil").exec()?;
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lua.gc_collect()));
            println!("gc_collect panicked: {}", r.is_err());
            println!("collectgarbage('count') = {:?}", lua.load("return collectgarbage('count')").eval::<Value>().map_err(|e| e.to_string()));
            let t = lua.create_table()?;
            println!("unprotected error now");
            println!("{:?}", t.set(Nil, 1).map_err(|e| e.to_string()));
            println!("end");
        }
        "status_normal" => {
            let chk = lua.create_function(|_, co: Thread| { println!("status of a coroutine that resumed us: {:?}", co.status()); Ok(()) })?;
            lua.globals().set("chk", chk)?;
            lua.load(r#"
                local outer
                outer = coroutine.create(function() print('lua says', coroutine.resume(coroutine.create(function() print(coroutine.status(outer)); chk(outer) end))) end)
                coroutine.resume(outer)
            "#).exec()?;
        }
        _ => panic!("unknown {which}"),
    }
    Ok(())
}
