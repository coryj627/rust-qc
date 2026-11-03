#![allow(unused)]
use mlua::prelude::*;
use mlua::{Nil, Value, Table, Function, Thread, UserData, AnyUserData, MultiValue};
use std::collections::{HashMap, HashSet, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
struct Bomb; impl Drop for Bomb { fn drop(&mut self) { panic!("boom in drop") } } impl UserData for Bomb {}
struct A(u64); struct B(String);
fn block_on<F: std::future::Future>(f: F) -> F::Output { futures_util::future::FutureExt::now_or_never(Box::pin(f)).expect("not ready") }
async fn genf(lua: Lua, _: ()) -> LuaResult<()> {
    for i in 0..5 { let v: MultiValue = lua.yield_with(i).await?; println!("resumed with {:?}", v.iter().map(|x| x.type_name()).collect::<Vec<_>>()); }
    Ok(())
}
fn main() -> LuaResult<()> {
    let a = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match a.as_str() {
        "nilkey" => { let t = lua.create_table()?; let r = t.set(Nil, 1); println!("{r:?}"); }
        "tablefrom" => { let r = lua.create_table_from([(f64::NAN, 1)]); println!("{r:?}"); }
        "tolua_nankey" => { #[derive(serde::Serialize)] struct K; let mut m = std::collections::BTreeMap::new(); m.insert((), 1); let r = lua.to_value_with(&m, mlua::SerializeOptions::new().serialize_unit_to_null(false)); println!("{r:?}"); }
        "bomb_collect" => {
            lua.create_userdata(Bomb)?;
            let r = catch_unwind(AssertUnwindSafe(|| lua.gc_collect())); println!("gc: {:?}", r.is_err());
            let r: LuaResult<i32> = lua.load("return 1+1").eval(); println!("after {r:?}");
            let r = lua.load("error('x')").exec(); println!("err {r:?}");
            let r = lua.load("local t={} for i=1,100000 do t[i]={} end return #t").eval::<i32>(); println!("alloc {r:?}");
        }
        "bomb_alloc" => {
            lua.create_userdata(Bomb)?;
            let r = catch_unwind(AssertUnwindSafe(|| lua.load("for i=1,1000000 do local t={} end").exec())); println!("run: {:?}", r.as_ref().map(|r| r.is_ok()).map_err(|_|"panic"));
            let r: LuaResult<i32> = lua.load("return 1+1").eval(); println!("after {r:?}");
            let r = lua.load("error('x')").exec(); println!("err {r:?}");
        }
        "bomb_close" => { lua.create_userdata(Bomb)?; drop(lua); println!("closed"); return Ok(()); }
        "warnabort" => { lua.set_warning_function(|_,_,_| Err(LuaError::runtime("w"))); lua.warning("m", false); println!("survived"); }
        "warnco" => {
            lua.set_warning_function(|_,m,_| {println!("warn {m}"); Err(LuaError::runtime("w"))});
            let r = lua.load("local co=coroutine.wrap(function() warn('x') print('after warn') return 7 end) print(pcall(co)) return 1").eval::<i32>(); println!("{r:?}");
        }
        "oomrep" => { let r = lua.load("return pcall(string.rep, 'x', 1<<50)").eval::<MultiValue>(); println!("{r:?}"); }
        "memshrink" => { let t: Table = lua.load("local t={} for i=1,100000 do t[i]=tostring(i) end return t").eval()?; lua.set_memory_limit(1024)?; let r = lua.gc_collect(); println!("{r:?}"); let r = t.raw_get::<i32>(1); println!("{r:?}"); lua.set_memory_limit(0)?; drop(t); println!("{:?}", lua.gc_collect()); }
        "normal_resume" => {
            // A resumes B; B's Rust callback resumes A
            let f = lua.create_function(|lua, a: Thread| { println!("status of A from B: {:?}", a.status()); let r = a.resume::<()>(()); println!("resume normal: {r:?}"); Ok(()) })?;
            lua.globals().set("rf", f)?;
            let r: LuaResult<()> = lua.load(r#"
               local A
               A = coroutine.create(function()
                  local x,y,z = 1,2,3
                  local B = coroutine.create(function() rf(A) end)
                  local ok, e = coroutine.resume(B)
                  print('B done', ok, e)
                  print('locals',x,y,z)
               end)
               print(coroutine.resume(A))
            "#).exec(); println!("{r:?}");
        }
        "normal_resume2" => {
            // A's Rust callback resumes B via Thread::resume; B resumes A
            let cb = lua.create_function(|lua, b: Thread| { let r = b.resume::<()>(()); println!("cb got {r:?}"); Ok(()) })?;
            let rf = lua.create_function(|lua, a: Thread| { println!("A status {:?}", a.status()); let r = a.resume::<()>(()); println!("resume normal: {r:?}"); Ok(()) })?;
            lua.globals().set("cb", cb)?; lua.globals().set("rf", rf)?;
            let r: LuaResult<()> = lua.load(r#"
               local A
               A = coroutine.create(function()
                  local B = coroutine.create(function() rf(A) end)
                  cb(B)
                  print('A continues')
               end)
               print(coroutine.resume(A))
            "#).exec(); println!("{r:?}");
        }
        "normal_reset" => {
            let cb = lua.create_function(|lua, b: Thread| { let r = b.resume::<()>(()); println!("cb got {r:?}"); Ok(()) })?;
            let rf = lua.create_function(|lua, a: Thread| { let f = lua.create_function(|_,()| Ok(()))?; let r = a.reset(f); println!("reset normal: {r:?}"); Ok(()) })?;
            lua.globals().set("cb", cb)?; lua.globals().set("rf", rf)?;
            let r: LuaResult<()> = lua.load(r#"
               local A
               A = coroutine.create(function()
                  local B = coroutine.create(function() rf(A) end)
                  cb(B)
                  print('A continues')
               end)
               print(coroutine.resume(A))
            "#).exec(); println!("{r:?}");
        }
        "main_reset" => {
            let main = lua.current_thread();
            let lua2 = lua.clone();
            let f = lua.create_function(move |lua, ()| { let f = lua.create_function(|_,()| Ok(()))?; println!("{:?}", main.status()); let r = main.reset(f); println!("{r:?}"); Ok(()) })?;
            lua.globals().set("rf", f)?;
            let th: Thread = lua.load("coroutine.create(function() rf() return 5 end)").eval()?;
            let r = th.resume::<i32>(()); println!("{r:?}");
            let v: i32 = lua.load("return 1").eval()?; println!("{v}");
        }
        "reset_bool" => {
            let th: Thread = lua.load("coroutine.create(function() error(true) end)").eval()?;
            let co: Function = lua.load("return coroutine.resume").eval()?;
            println!("{:?}", co.call::<MultiValue>(&th)?);
            let f = lua.create_function(|_,()| Ok(()))?;
            println!("{:?}", th.reset(f));
        }
        "setroundtrip" => {
            let s: HashSet<i32> = [1,2,3].into_iter().collect();
            let v = lua.pack(s)?; let r: LuaResult<HashSet<i32>> = lua.unpack(v); println!("{r:?}");
            let s: BTreeSet<i32> = [1,2].into_iter().collect();
            let v = lua.pack(s)?; let r: LuaResult<BTreeSet<i32>> = lua.unpack(v); println!("{r:?}");
        }
        "typeconf" => {
            lua.register_userdata_type::<A>(|_|{})?;
            let ud = lua.create_any_userdata(A(0x4141414141414141))?;
            ud.borrow::<A>()?;
            lua.register_userdata_type::<A>(|_|{})?;
            drop(ud); lua.gc_collect()?; lua.gc_collect()?;
            let ud2 = lua.create_any_userdata(B("hello world this is a string".into()))?;
            println!("is A? {} is B? {}", ud2.is::<A>(), ud2.is::<B>());
            match ud2.borrow::<A>() { Ok(a) => println!("CONFUSED: {:x}", a.0), Err(e) => println!("ok: {e}") }
        }
        "yieldnew" => {
            lua.globals().set("gen", lua.create_async_function(genf)?)?;
            lua.globals().set("fail", lua.create_function(|_,()| -> LuaResult<()> { Err(LuaError::runtime("fail")) })?)?;
            let r = lua.load(r#"
              local co = coroutine.wrap(gen)
              print('y', co())
              print(pcall(fail))
              print('y', co('x'))
              print(pcall(fail))
              print('y', co('x'))
              print(pcall(fail))
              print(pcall(fail))
              print('y', co('x','y'))
              collectgarbage()
              print(pcall(fail))
              print(pcall(fail))
              print(pcall(fail))
            "#).exec(); println!("{r:?}");
        }
        "refexhaust" => {
            let r: LuaResult<Vec<Value>> = lua.load("local t={} for i=1,1100000 do t[i]=tostring(i) end return t").eval(); println!("{:?}", r.map(|v| v.len()));
        }
        "refexhaust_cb" => {
            lua.globals().set("f", lua.create_function(|_, v: Vec<Value>| Ok(v.len()))?)?;
            let r: LuaResult<MultiValue> = lua.load("local t={} for i=1,1100000 do t[i]=tostring(i) end return pcall(f, t)").eval(); println!("{:?}", r.map(|v| v.len()));
        }
        "dbgref" => {
            let t: Table = lua.load("local t={} for i=1,1100000 do t['k'..i]=i end return t").eval()?; let s = format!("{t:#?}"); println!("{}", s.len());
        }
        "nullskip" => {
            let t: Value = lua.load("return {1, null, 3}").set_environment(lua.create_table_from([("null", lua.null())])?).eval()?;
            println!("default: {}", serde_json::to_string(&t.to_serializable()).unwrap());
            println!("deny=false: {}", serde_json::to_string(&t.to_serializable().deny_unsupported_types(false)).unwrap());
            let v: Vec<Option<i32>> = lua.from_value_with(t.clone(), mlua::DeserializeOptions::new().deny_unsupported_types(false))?; println!("{v:?}");
            let t2: Value = lua.load("return {a=null, b=2}").set_environment(lua.create_table_from([("null", lua.null())])?).eval()?;
            println!("map deny=false: {}", serde_json::to_string(&t2.to_serializable().deny_unsupported_types(false)).unwrap());
        }
        "sparse" => {
            let t: Value = lua.load("return {[1000000000000]=1}").eval()?;
            let s = serde_json::to_string(&t.to_serializable().detect_mixed_tables(true));
            println!("{:?}", s.map(|s| s.len()));
        }
        "sparse_huge" => {
            let t: Value = lua.load("return {[math.huge]=1}").eval()?;
            let s = serde_json::to_string(&t.to_serializable().detect_mixed_tables(true));
            println!("{:?}", s.map(|s| s.len()));
        }
        "deepser" => { let t: Value = lua.load("local t={} for i=1,1000000 do t={t} end return t").eval()?; let s = serde_json::to_string(&t); println!("{:?}", s.map(|s| s.len())); }
        "deepdbg" => { let t: Value = lua.load("local t={} for i=1,1000000 do t={t} end return t").eval()?; let s = format!("{t:#?}"); println!("{}", s.len()); }
        "deepde" => { let t: Value = lua.load("local t={} for i=1,1000000 do t={t} end return t").eval()?; let s: LuaResult<serde_json::Value> = lua.from_value(t); println!("{}", s.is_ok()); }
        "deepto" => { let mut v = serde_json::json!(1); for _ in 0..200000 { v = serde_json::Value::Array(vec![v]); } let r = lua.to_value(&v); println!("{}", r.is_ok()); std::mem::forget(v); }
        "lenoverflow" => {
            let t: Table = lua.load("return setmetatable({}, {__len=function() return math.maxinteger end, __newindex=function() end})").eval()?;
            let r = t.push(1); println!("{r:?}");
        }
        "popempty" => {
            let t = lua.create_table()?; let r: LuaResult<Value> = t.raw_pop(); println!("{r:?}");
            let n = lua.create_table()?; n.raw_set(0, 5)?; let r: LuaResult<i32> = n.raw_pop(); println!("raw_pop on {{[0]=5}} -> {r:?}, t[0] now {:?}", n.raw_get::<Value>(0)?);
            let r: LuaResult<i32> = n.pop();
        }
        "hookshift" => {
            lua.set_hook(mlua::HookTriggers::EVERY_LINE, |lua, d| { let _ = lua.create_table(); Ok(mlua::VmState::Continue) })?;
            let r: LuaResult<i32> = lua.load("local a,b,c = 1,2,3\nlocal d = a+b+c\nreturn d").eval(); println!("{r:?}");
        }

        "bomb_state" => {
            println!("gc running before: {}", lua.gc_is_running());
            lua.create_userdata(Bomb)?;
            let r = catch_unwind(AssertUnwindSafe(|| lua.gc_collect())); println!("gc panicked: {:?}", r.is_err());
            println!("gc running after: {}", lua.gc_is_running());
            let before = lua.used_memory();
            lua.load("for i=1,300000 do local t={} end").exec()?;
            println!("used before {before} after loop {}", lua.used_memory());
            let t = lua.create_table()?;
            println!("now unprotected error:");
            let r = t.set(Nil, 1); println!("{r:?}");
        }
        "bomb_cb" => {
            // panic-through-GC inside a callback, then keep going
            lua.create_userdata(Bomb)?;
            let f = lua.create_function(|lua, ()| { let r = catch_unwind(AssertUnwindSafe(|| lua.gc_collect())); println!("inner caught: {}", r.is_err()); Ok(1) })?;
            lua.globals().set("f", f)?;
            let r: LuaResult<i32> = lua.load("local a = f() error('after')").eval(); println!("{r:?}");
            let r: LuaResult<i32> = lua.load("return 1").eval(); println!("{r:?}");
        }
        "typeconf2" => {
            macro_rules! try_b { ($($n:literal)*) => { $( {
                struct Bn<const N: usize>(String);
                lua.register_userdata_type::<A>(|_|{})?;
                let ud = lua.create_any_userdata(A(0x4141414141414141))?;
                ud.borrow::<A>()?;
                lua.register_userdata_type::<A>(|_|{})?;
                drop(ud); for _ in 0..4 { lua.gc_collect()?; }
                let ud2 = lua.create_any_userdata(Bn::<$n>("hello".into()))?;
                if ud2.is::<A>() { println!("N={}: metatable reused: is::<A>() true on a Bn userdata", $n);
                    match ud2.borrow::<A>() { Ok(a) => { println!("CONFUSED read: {:x}", a.0); return Ok(()); } Err(e) => println!("borrow err {e}") }
                }
            } )* } }
            try_b!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31);
            println!("no reuse hit");
        }
        "oomulimit" => {
            let r = lua.load("local s=string.rep('x',1<<28) local t={} for i=1,40 do t[i]=s..i end return #t").eval::<i32>(); println!("{r:?}");
        }
        "gcgen_in_gc" => {
            let f = lua.create_function(|lua, ()| { let m = lua.gc_gen(0,0); Ok(format!("{m:?}")) })?;
            lua.globals().set("f", f)?;
            let r = lua.load("setmetatable({}, {__gc=function() print(pcall(f)) end}) collectgarbage() collectgarbage()").exec(); println!("{r:?}");
        }
        "ud_eq_dead" => {
            let ud = lua.create_any_userdata(A(1))?;
            let v = Value::UserData(ud.clone());
            ud.destroy()?;
            println!("{:?}", v.to_string());
            println!("{:#?}", v);
        }
        "emptykey" => {
            let t: Table = lua.load("return {['']=1, x=2}").eval()?;
            println!("{t:#?}");
        }
        "sharedfmt" => {
            let t: Table = lua.load("local s={1} return {a=s,b=s}").eval()?;
            println!("{t:#?}");
        }
        "seqmix" => {
            let t: Table = lua.load("return {1,2,nil,4}").eval()?;
            println!("{:?}", t.sequence_values::<Value>().count());
            println!("{}", t == [1,2]);
            println!("{}", t == [1,2,99]);
        }
        "chunkname" => {
            let r = lua.load("error('x')").set_name("a\0b").exec(); println!("{r:?}");
        }
        "bindmany" => {
            let f: Function = lua.load("return select('#', ...)").into_function()?;
            let args: Vec<i32> = (0..300).collect();
            let r = f.bind(mlua::Variadic::from(args)); println!("{:?}", r.map(|_| ()));
            let args: Vec<i32> = (0..254).collect();
            let g = f.bind(mlua::Variadic::from(args))?; println!("{:?}", g.call::<i32>(()));
        }
        "bindnil" => {
            let f: Function = lua.load("return select('#', ...)").into_function()?;
            let g = f.bind((Nil, Nil))?.bind(Nil)?; println!("{:?}", g.call::<i32>((Nil,)));
        }
        "valuelen" => {
            let r: LuaResult<Value> = lua.load("return 1").eval(); println!("{r:?}");
            let e: LuaResult<Value> = lua.load("").eval(); println!("{e:?}");
            let bad: LuaResult<MultiValue> = lua.load("return table.unpack({}, 1, 1000000)").eval(); println!("{:?}", bad.map(|m|m.len()));
            let f = lua.create_function(|_, v: mlua::Variadic<i32>| Ok(v.len()))?; lua.globals().set("f", f)?;
            let r: LuaResult<usize> = lua.load("return f(table.unpack({}, 1, 900000))").eval(); println!("{r:?}");
            let mv: MultiValue = (0..2_000_000).map(Value::Integer).collect();
            let g: Function = lua.load("return select('#', ...)").into_function()?;
            println!("{:?}", g.call::<i32>(mv));
        }
        "seqnilstack" => {
            // deep recursion of Rust callback through lua, then Table API at the stack limit
            let f = lua.create_function(|lua, t: Table| {
                 let v = lua.create_sequence_from(0..10)?; let g = lua.globals(); let _ = t.raw_len(); Ok(v.raw_len())
            })?; lua.globals().set("f", f)?;
            let r: LuaResult<i32> = lua.load("local function r(n) f({}) return 1 + r(n+1) end return r(0)").eval(); println!("{:?}", r.map_err(|e| e.to_string().chars().take(100).collect::<String>()));
        }

        "typeconf3" => {
            fn addr<T: std::fmt::Debug>(t: &T) -> usize { let s = format!("{t:?}"); let i = s.find("0x").unwrap(); let e = s[i+2..].find(|c: char| !c.is_ascii_hexdigit()).unwrap()+i+2; usize::from_str_radix(&s[i+2..e],16).unwrap() }
            lua.register_userdata_type::<A>(|_|{})?;
            let ud = lua.create_any_userdata(A(0x4141414141414141))?;
            ud.borrow::<A>()?;
            let old_mt = addr(&ud.metatable()?);
            println!("old mt {old_mt:x}");
            lua.register_userdata_type::<A>(|_|{})?;
            drop(ud);
            for _ in 0..6 { lua.gc_collect()?; }
            // find a table allocated at the freed address
            let mut spray = Vec::new();
            let mut hit = None;
            for i in 0..200000 { let t = lua.create_table()?; if t.to_pointer() as usize == old_mt { hit = Some(t); println!("spray hit after {i}"); break; } spray.push(t); }
            let hit = hit.expect("no reuse");
            lua.gc_stop();
            drop(hit); lua.gc_restart();
            for _ in 0..3 { lua.gc_collect()?; }
            lua.gc_stop();
            let ud2 = lua.create_any_userdata(B("hello world this is a string".into()))?;
            let new_mt = addr(&ud2.metatable()?);
            println!("new mt {new_mt:x}, same={}", new_mt==old_mt);
            println!("is A? {} is B? {}", ud2.is::<A>(), ud2.is::<B>());
            match ud2.borrow::<A>() { Ok(a) => println!("CONFUSED: {:x}", a.0), Err(e) => println!("ok: {e}") }
        }
        "sparse2" => {
            let t: Value = lua.load("return {[50000000]=1}").eval()?;
            let s = serde_json::to_string(&t.to_serializable().detect_mixed_tables(true));
            println!("{:?}", s.map(|s| s.len()));
        }
        "scope_ud_err" => {
            struct S; impl UserData for S {}
            let r = lua.scope(|scope| { let ud = scope.create_userdata(S)?; lua.globals().set("ud", &ud)?; ud.destroy() }); println!("{r:?}");
            let r: LuaResult<Value> = lua.load("return tostring(ud)").eval(); println!("{r:?}");
        }
        "scope_fn_escape" => {
            let mut n = 0;
            let r: LuaResult<Function> = lua.scope(|scope| { let f = scope.create_function_mut(|_, ()| { n+=1; Ok(n) })?; f.call::<i32>(())?; Ok(f) });
            let f = r?; println!("{:?}", f.call::<i32>(()).map_err(|e| e.to_string()));
        }
        "scope_recursive_drop" => {
            // scoped callback that drops/closes while executing: call scope fn in nested coroutine that outlives scope
            let th: Thread = lua.scope(|scope| {
                let f = scope.create_function(|lua, ()| { let y: Function = lua.load("return coroutine.yield").eval()?; y.call::<()>(())?; Ok(()) })?;
                let th = lua.create_thread(f)?;
                println!("{:?}", th.resume::<()>(()).map_err(|e| e.to_string()));
                Ok(th)
            })?;
            println!("{:?}", th.status());
        }
        "ud_ref_outlive" => {
            let ud = lua.create_any_userdata(String::from("hello"))?;
            let r = ud.borrow::<String>()?;
            println!("destroy: {:?}", ud.destroy());
            println!("still: {}", &*r);
            println!("borrow after destroy: {:?}", ud.borrow::<String>().map(|s| s.len()));
            drop(ud);
            for _ in 0..3 {lua.gc_collect()?;}
            drop(lua);
            println!("after lua drop: {}", &*r);
        }
        "ud_take_shared" => {
            let ud = lua.create_any_userdata(String::from("hello"))?;
            let r = ud.borrow::<String>()?; 
            println!("{:?}", ud.take::<String>()); println!("{}", &*r);
        }
        "ud_method_self_destroy" => {
            struct M(Vec<u8>); impl UserData for M { fn add_methods<T: mlua::UserDataMethods<Self>>(m: &mut T) {
                m.add_function("d", |lua, ud: AnyUserData| { let r = ud.destroy(); println!("destroy in fn: {r:?}"); Ok(()) });
                m.add_method("kill", |lua, this, ud: AnyUserData| { let r = ud.destroy(); println!("destroy in method: {r:?}"); let r = ud.take::<M>().map(|_|()); println!("take in method: {r:?}"); println!("{}", this.0.len()); Ok(()) });
            } }
            let ud = lua.create_userdata(M(vec![1,2,3]))?; lua.globals().set("u", &ud)?;
            let r = lua.load("u:kill(u) u.d(u)").exec(); println!("{r:?}");
        }
        "pairs_during_callback" => {
            let t: Table = lua.load("return {a=1}").eval()?;
            // TablePairs holds a LuaGuard; keep it across a coroutine
            let mut it = t.pairs::<String,i32>();
            println!("{:?}", it.next());
            drop(it);
        }
        "seq_stack" => {
            // TableSequence check_stack(1) but needs 2 + conversion; run at the limit
            let f = lua.create_function(|lua, t: Table| {
                 let mut c = 0; for v in t.sequence_values::<Value>() { v?; c+=1; } Ok(c)
            })?; lua.globals().set("f", f)?;
            let r: LuaResult<i32> = lua.load("local t={1,2,3} local function r(n) f(t) return 1 + r(n+1) end return r(0)").eval(); println!("{:?}", r.map_err(|e| e.to_string().chars().take(80).collect::<String>()));
        }

        "typeconf4" => {
            struct A2(u64); struct B2([u64;32]);
            lua.register_userdata_type::<A2>(|_|{})?;
            let ud = lua.create_any_userdata(A2(0x4141414141414141))?;
            ud.borrow::<A2>()?;
            lua.register_userdata_type::<A2>(|_|{})?;
            drop(ud);
            for _ in 0..4 { lua.gc_collect()?; }
            let ud2 = lua.create_any_userdata(B2([0x4242424242424242;32]))?;
            println!("is A2? {} is B2? {}", ud2.is::<A2>(), ud2.is::<B2>());
            match ud2.borrow::<A2>() { Ok(a) => println!("CONFUSED: B2 data read as A2: {:x}", a.0), Err(e) => println!("ok: {e}") }
        }
        "typeconf5" => {
            // same, but the confused type holds a heap pointer: A3 is a String
            struct B3([u64;32]);
            lua.register_userdata_type::<String>(|_|{})?;
            let ud = lua.create_any_userdata(String::from("x"))?;
            ud.borrow::<String>()?;
            lua.register_userdata_type::<String>(|_|{})?;
            drop(ud);
            for _ in 0..4 { lua.gc_collect()?; }
            let ud2 = lua.create_any_userdata(B3([0x4242424242424242;32]))?;
            match ud2.borrow::<String>() { Ok(a) => println!("CONFUSED: {}", a.len()), Err(e) => println!("ok: {e}") }
            let ud3 = ud2.borrow::<String>()?; println!("{}", &*ud3);
        }
        "clearforeach" => {
            let t: Table = lua.load("return {a=1,b=2,c=3}").eval()?;
            let other = lua.create_table()?;
            let r = t.for_each(|k: Value, v: Value| { println!("{k:?} {v:?}"); other.clear() });
            println!("{r:?}");
        }
        "clearcb" => {
            // leak in a callback: arguments indexing after clear
            let f = lua.create_function(|lua, (t,x): (Table,i32)| { t.clear()?; Ok(x) })?;
            lua.globals().set("f", f)?;
            println!("{:?}", lua.load("return f({1,2},7)").eval::<i32>());
            let t = lua.create_table()?; t.clear()?;
            // stack guard above: leak on the main stack makes Thread::resume etc see extra values
            let th: Thread = lua.load("coroutine.create(function(...) return select('#',...) end)").eval()?;
            println!("{:?}", th.resume::<i32>(1));
            // ScopeGuard nesting: pairs iterator with clear
            let u: Table = lua.load("return {a=1,b=2,c=3,d=4}").eval()?;
            let o = lua.create_table()?;
            let mut n = 0; for p in u.pairs::<String,i32>() { p?; o.clear()?; n+=1; } println!("pairs n={n}");
        }
        "clearseq" => {
            // for_each_value (public hidden) and sequence with leak
            let t: Table = lua.load("return {1,2,3}").eval()?;
            let o = lua.create_table()?;
            let r = t.for_each_value::<i32>(|v| { println!("{v}"); o.clear() }); println!("{r:?}");
        }
        "pairs_rehash_in_cb" => {
            let f = lua.create_function(|lua, t: Table| {
                for p in t.pairs::<String,i32>() { let (k,_) = p?; t.set(k, Nil)?; for j in 0..64 { t.set(format!("x{j}"), 1)?; t.set(format!("x{j}"), Nil)?; } lua.gc_collect()?; }
                println!("loop done");
                Ok(())
            })?;
            lua.globals().set("f", f)?;
            let r = lua.load("local t={} for i=1,8 do t['k'..i]=i end print(pcall(f,t)) return 1").eval::<i32>(); println!("{r:?}");
        }

        "call_many_ud" => {
            let ud = lua.create_any_userdata(1i32)?;
            let f: Function = lua.load("local u=... return 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,u").into_function()?;
            let r: LuaResult<MultiValue> = f.call(&ud); println!("{:?}", r.map(|m| m.len()));
            let f: Function = lua.load("local u=... return u").into_function()?;
            let r: LuaResult<Value> = f.call(&ud); println!("{:?}", r.map(|v| v.type_name()));
            let g: Function = lua.load("local t=... return table.unpack(t)").into_function()?;
            let vals: Table = lua.create_sequence_from((0..200).map(|_| ud.clone()))?;
            let r: LuaResult<MultiValue> = g.call(vals); println!("{:?}", r.map(|m| m.len()));
        }
        "thread_many_ud" => {
            let ud = lua.create_any_userdata(1i32)?;
            let th: Thread = lua.load("coroutine.create(function(u) return 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,u end)").eval()?;
            let r: LuaResult<MultiValue> = th.resume(&ud); println!("{:?}", r.map(|m| m.len()));
        }
        "weak_after_drop" => {
            let t = lua.create_table()?;
            let s = lua.create_string("x")?;
            drop(lua);
            println!("{:?}", t.to_pointer());
        }
        "string_after_drop" => {
            let s = lua.create_string("x")?;
            drop(lua);
            println!("{:?}", s.as_bytes().len());
        }
        "equals_diff_lua" => {
            let lua2 = Lua::new();
            let a = lua.create_table()?; let b = lua2.create_table()?;
            println!("{}", a==b);
        }
        "set_diff_lua" => {
            let lua2 = Lua::new();
            let a = lua.create_table()?; let b = lua2.create_table()?;
            println!("{:?}", a.set("x", b));
        }
        "sortcmp2" => {
            // non-total comparator: integers near 2^63 vs float 2^63 keys
            let mut pan = 0;
            for seed in 0..20 {
              let src = format!("math.randomseed({seed}) local t={{}} for i=1,4000 do local r=math.random() if r<0.5 then t[math.maxinteger-math.random(0,5000)]=i else t[2^63*(1+math.random(0,3)/4)]=i; t['s'..i]=i end end return t");
              let t: Table = lua.load(src).eval()?;
              let r = catch_unwind(AssertUnwindSafe(|| format!("{t:#?}").len()));
              if r.is_err() { pan+=1; }
              let v = Value::Table(t);
              let r = catch_unwind(AssertUnwindSafe(|| serde_json::to_string(&v.to_serializable().sort_keys(true))));
              if r.is_err() { pan+=1; }
            }
            println!("panics {pan}");
        }
        "ser_seqlen" => {
            // declared seq len vs elements emitted when skipping
            let t: Value = lua.load("return {1, print, 3}").eval()?;
            struct Chk; 
            println!("{:?}", serde_json::to_string(&t.to_serializable().deny_unsupported_types(false)));
        }
        "de_borders" => {
            let t: Value = lua.load("return {1,2,nil,4}").eval()?;
            println!("{:?}", lua.from_value::<serde_json::Value>(t.clone()));
            println!("{:?}", serde_json::to_string(&t));
            let m: Value = lua.load("return {1,2,a=3}").eval()?;
            println!("mixed: {:?}", lua.from_value::<serde_json::Value>(m.clone()));
            println!("mixed: {:?}", serde_json::to_string(&m));
            println!("mixed detect: {:?}", serde_json::to_string(&m.to_serializable().detect_mixed_tables(true)));
            let e: Value = lua.load("return {}").eval()?;
            println!("empty: {:?}", serde_json::to_string(&e));
            let n0: Value = lua.load("return {[0]=1,[1]=2}").eval()?;
            println!("zero: {:?}", serde_json::to_string(&n0.to_serializable().detect_mixed_tables(true)));
            let fl: Value = lua.load("return {[1.5]=1}").eval()?;
            println!("float key: {:?}", serde_json::to_string(&fl));
            let tk: Value = lua.load("return {[{}]=1}").eval()?;
            println!("table key: {:?}", serde_json::to_string(&tk));
            let bk: Value = lua.load("return {[true]=1}").eval()?;
            println!("bool key: {:?}", serde_json::to_string(&bk));
            let ik: Value = lua.load("return {[10]=1,[20]=2}").eval()?;
            println!("int key: {:?}", serde_json::to_string(&ik));
            println!("int key from_value: {:?}", lua.from_value::<serde_json::Value>(ik.clone()));
            println!("hash int: {:?}", lua.from_value::<HashMap<i32,i32>>(ik.clone()));
            let st: Value = lua.load("return '\\255'").eval()?;
            println!("bytes: {:?}", serde_json::to_string(&st));
            let big: Value = lua.load("return 2^53").eval()?; 
            println!("i64 from float: {:?} {:?}", lua.from_value::<i64>(big.clone()), lua.from_value::<u8>(Value::Integer(300)));
            println!("f32 from int: {:?}", lua.from_value::<f32>(Value::Integer(3)));
            println!("bool from int: {:?}", lua.from_value::<bool>(Value::Integer(1)));
            println!("opt null: {:?}", lua.from_value::<Option<i32>>(Value::NULL));
            println!("unit null: {:?}", lua.from_value::<()>(Value::NULL));
            println!("string: {:?}", lua.from_value::<String>(Value::Integer(3)));
        }
        "to_value" => {
            let v = serde_json::json!({"a": null, "b":[1,null,3], "c":{}});
            let lv = lua.to_value(&v)?;
            println!("{:?}", serde_json::to_string(&lv));
            let rt: serde_json::Value = lua.from_value(lv)?; println!("{rt}");
            let u: Value = lua.to_value(&(1u64<<63))?; println!("{u:?}");
            let i: Value = lua.to_value(&i128::MAX)?; println!("{i:?}");
            #[derive(serde::Serialize, serde::Deserialize, Debug)] enum E { A, B(i32), C(i32,i32), D{x:i32} }
            for e in [E::A,E::B(1),E::C(1,2),E::D{x:1}] { let v = lua.to_value(&e)?; println!("{:?} -> {:?}", e, lua.from_value::<E>(v)); }
            let c = lua.to_value(&'x')?; println!("{:?}", lua.from_value::<char>(c));
            let nested: Vec<Vec<i32>> = vec![vec![],vec![1]]; let nv = lua.to_value(&nested)?; println!("{:?}", lua.from_value::<Vec<Vec<i32>>>(nv));
            let m: HashMap<i32,String> = [(1,"a".into()),(2,"b".into())].into_iter().collect(); let mv = lua.to_value(&m)?; println!("{:?}", lua.from_value::<HashMap<i32,String>>(mv.clone())); println!("{:?}", lua.from_value::<std::collections::BTreeMap<String,String>>(mv));
        }
        "path" => {
            let t: Table = lua.load("return {a={b={[1]={c=5}}, ['x.y']=1, ['q\"r']=2, [-1]=3}, [\"\"]=9}").eval()?;
            for p in ["a.b[1].c","a['x.y']",r#"a["q\"r"]"#,"a[-1]","a.zz?.c","a.zz.c","","a[-9223372036854775808]","  a . b ","a.b [1]","a?","é.a","a.b[1]?"] { println!("{p:?} -> {:?}", t.get_path::<Value>(p).map_err(|e| e.to_string())); }
        }

        "scope_uaf" => {
            struct S(String);
            impl Drop for S { fn drop(&mut self) { println!("S dropped"); self.0.clear(); self.0.shrink_to_fit(); } }
            impl UserData for S { fn add_methods<T: mlua::UserDataMethods<Self>>(m: &mut T) {
                m.add_method("get", |_, this, ()| { println!("method running on S: {:?} cap {}", this.0.len(), this.0.capacity()); Ok(this.0.clone()) });
                m.add_method_mut("set", |_, this, v: String| { println!("mut method running"); this.0.push_str(&v); Ok(()) });
            } }
            lua.scope(|scope| {
                let ud = scope.create_userdata(S("hello scoped".repeat(4)))?;
                lua.globals().set("ud", &ud)?;
                lua.load("m = ud.get; n = ud.set; print('in scope', m(ud))").exec()?;
                Ok(())
            })?;
            println!("scope ended");
            let spray: Vec<String> = (0..64).map(|i| format!("{:032}", i)).collect();
            let r = lua.load("return m(ud)").eval::<LuaString>(); println!("after scope get: {:?}", r.map(|s| s.to_string_lossy()));
            let r = lua.load("n(ud, 'pwn') return 1").eval::<i32>(); println!("after scope set: {:?}", r);
            drop(spray);
        }
        "scope_uaf_any" => {
            let data = vec![1u8,2,3];
            lua.scope(|scope| {
                let ud = scope.create_any_userdata(data, |r| { r.add_method("len", |_, this: &Vec<u8>, ()| { println!("ptr {:?}", this.as_ptr()); Ok(this.iter().map(|x| *x as u64).sum::<u64>()) }); })?;
                lua.globals().set("ud", &ud)?;
                lua.load("m = ud.len print(m(ud))").exec()?;
                Ok(())
            })?;
            let r = lua.load("return m(ud)").eval::<u64>(); println!("after scope: {:?}", r);
        }
        "scope_ref_shared" => {
            struct S2(String); impl UserData for S2 { fn add_methods<T: mlua::UserDataMethods<Self>>(m: &mut T) { m.add_method("get", |_, this, ()| Ok(this.0.clone())); } }
            let d = S2("x".into());
            lua.scope(|scope| { let ud = scope.create_userdata_ref(&d)?; lua.globals().set("ud", &ud)?; lua.load("m=ud.get print(m(ud))").exec()?; Ok(()) })?;
            let r = lua.load("return m(ud)").eval::<String>(); println!("{:?}", r.map_err(|e| e.to_string().chars().take(100).collect::<String>()));
        }
        "ud_eq_foreign" => {
            let a: Value = lua.load("return io.stdout").eval()?; let b: Value = lua.load("return io.stderr").eval()?;
            println!("{:?}", a.equals(&b)); println!("{:?}", a.equals(&a));
            struct E1; impl UserData for E1 {} struct E2; impl UserData for E2 { fn add_methods<T: mlua::UserDataMethods<Self>>(m: &mut T) { m.add_meta_method(mlua::MetaMethod::Eq, |_, _, _: Value| Ok(true)); } }
            let x = Value::UserData(lua.create_userdata(E1)?); let y = Value::UserData(lua.create_userdata(E2)?);
            lua.globals().set("x", x.clone())?; lua.globals().set("y", y.clone())?;
            println!("mlua {:?} lua {:?}", x.equals(&y), lua.load("return x==y").eval::<bool>());
        }
        "bytecode_fuzz" => {
            let r = lua.load(r#"
               local d = string.dump(function(a) local t = {a,1,2} return t[1] + #t end)
               local n=0
               math.randomseed(1)
               for i=1,200000 do
                 local p = math.random(20,#d)
                 local s = d:sub(1,p-1)..string.char(math.random(0,255))..d:sub(p+1)
                 local f = load(s,'x','b')
                 if f then pcall(f, 1) n=n+1 end
               end
               return n
            "#).eval::<i64>(); println!("{r:?}");
        }
        "settypemt" => {
            let mt = lua.create_table()?; mt.set("__index", lua.create_function(|_, ()| Ok(42))?)?;
            lua.set_type_metatable::<Table>(Some(mt));
            println!("{:?}", lua.load("return ({}).x").eval::<Value>());
        }

        "destroy_uaf" => {
            use std::cell::RefCell;
            struct M(Vec<u8>);
            impl Drop for M { fn drop(&mut self) { println!("M dropped"); } }
            thread_local!{ static HOLD: RefCell<Option<mlua::UserDataRef<M>>> = RefCell::new(None); }
            impl UserData for M { fn add_methods<T: mlua::UserDataMethods<Self>>(m: &mut T) {
                m.add_method("kill", |_, this, ud: AnyUserData| {
                    println!("destroy: {:?}", ud.destroy());
                    HOLD.with(|h| h.borrow_mut().take());
                    println!("after free: len {} ptr {:?}", this.0.len(), this.0.as_ptr());
                    let spray: Vec<Vec<u8>> = (0..64).map(|_| vec![0x41u8; 48]).collect();
                    println!("after spray: len {} ", this.0.len());
                    println!("sum {}", this.0.iter().map(|x| *x as u64).sum::<u64>());
                    std::mem::forget(spray);
                    Ok(())
                });
            } }
            let ud = lua.create_userdata(M(vec![1,2,3]))?;
            HOLD.with(|h| *h.borrow_mut() = Some(ud.borrow::<M>().unwrap()));
            lua.globals().set("u", &ud)?;
            let r = lua.load("u:kill(u)").exec(); println!("{r:?}");
        }
        "hook_shift" => {
            lua.globals().set("fail", lua.create_function(|_,()| -> LuaResult<()> { Err(LuaError::runtime("fail")) })?)?;
            lua.set_hook(mlua::HookTriggers::EVERY_LINE, |lua, d| {
                let g: Option<Function> = lua.globals().get("G")?;
                if let Some(g) = g { println!("line {:?} G()={:?}", d.current_line(), g.call::<Value>(())); }
                Ok(mlua::VmState::Continue)
            })?;
            let r = lua.load("local a,b,c = 111,222,333\nG=function() return b end\nprint(pcall(fail))\nlocal x = 1\nlocal y = 2\nreturn a+b+c").eval::<i32>(); println!("{r:?}");
        }
        "async_drop_runs" => {
            lua.globals().set("mark", lua.create_function(|_, s: String| { println!("lua continued after drop: {s}"); Ok(()) })?)?;
            let f: Function = lua.load("local x = coroutine.yield() mark(tostring(x)) local y = coroutine.yield() mark('second '..tostring(y))").into_function()?;
            {
                let mut fut = Box::pin(f.call_async::<()>(()));
                let w = futures_util::task::noop_waker();
                let mut cx = std::task::Context::from_waker(&w);
                println!("poll: {:?}", std::future::Future::poll(fut.as_mut(), &mut cx).is_pending());
                drop(fut);
            }
            println!("dropped");
        }
        "reqs" => {}

        "xlua_mt" => {
            let lua2 = Lua::new();
            let ss: Vec<LuaString> = (0..64).map(|i| lua.create_string(format!("not a table {i}")).unwrap()).collect();
            let t = lua.create_table()?;
            let mts: Vec<Table> = (0..32).map(|_| lua2.create_table().unwrap()).collect();
            lua.globals().set("t", &t)?;
            for m in &mts { let r = t.set_metatable(Some(m.clone())); }
            println!("set_metatable with another state's table returned Ok; has_metatable={}", t.has_metatable());
            let r = lua.load("return t.x").eval::<Value>(); println!("{:?}", r.map(|v| v.type_name()));
            println!("{:?}", t.metatable().map(|m| m.raw_len()));
        }
        "xlua_thread" => {
            let lua2 = Lua::new();
            let ss: Vec<LuaString> = (0..32).map(|i| lua.create_string(format!("s{i}")).unwrap()).collect();
            let f2: Vec<Function> = (0..16).map(|_| lua2.create_function(|_,()| Ok(1)).unwrap()).collect();
            let th = lua.create_thread(f2[3].clone()); println!("{:?}", th.as_ref().map(|t| t.status()));
            let r = th?.resume::<Value>(()); println!("{:?}", r.map_err(|e| e.to_string().chars().take(80).collect::<String>()));
        }
        "seq_under" => {
            let f: Function = lua.load("return {1,2,3},2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30").into_function()?;
            let r: LuaResult<Vec<i32>> = f.call(()); println!("{r:?}");
        }
        "env_under" => {
            let g: Function = lua.load("return 1").into_function()?;
            let f: Function = lua.load("local g=... return g,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30").into_function()?;
            struct E; impl FromLua for E { fn from_lua(v: Value, _: &Lua) -> LuaResult<E> { if let Value::Function(f) = v { println!("{:?}", f.environment().is_some()); } Ok(E) } }
            let r: LuaResult<E> = f.call(&g); println!("{:?}", r.is_ok());
        }
        "path_fuzz" => {
            let t: Table = lua.load("return {a={['a']=1}}").eval()?;
            let alpha: Vec<char> = "a.[]'\"\\?-0 é".chars().collect();
            let mut n = 0u64; 
            for len in 0..=5 { let mut idx = vec![0usize; len]; loop { let s: String = idx.iter().map(|&i| alpha[i]).collect(); let _ = t.get_path::<Value>(&s); n+=1;
                let mut k = 0; while k < len { idx[k]+=1; if idx[k]<alpha.len() {break;} idx[k]=0; k+=1; } if k==len {break;} } }
            println!("no panic over {n} inputs");
        }
        _ => panic!("unknown"),
    }
    Ok(())
}
