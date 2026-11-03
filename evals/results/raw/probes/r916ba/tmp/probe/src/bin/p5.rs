use mlua::prelude::*;
use mlua::{Table, Value, HookTriggers, VmState, ObjectLike, UserData, UserDataMethods, UserDataFields, MetaMethod};

struct Ud(i32);
impl UserData for Ud {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) { f.add_field_method_get("v", |_, t| Ok(t.0)); }
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("get", |_, t, ()| Ok(t.0));
        m.add_meta_method(MetaMethod::ToString, |_, t, ()| Ok(format!("Ud({})", t.0)));
    }
}

fn ops(lua: &Lua) -> LuaResult<()> {
    let t = lua.create_table()?;
    t.set("k", "value string")?;
    t.raw_set(1, 2)?;
    t.push(3)?;
    let _: Option<i32> = t.pop()?;
    let s = lua.create_string("some string to intern, long enough to allocate more than short strings")?;
    t.set(s.clone(), s)?;
    let f = lua.create_function(|lua, a: i32| { let t = lua.create_table()?; t.set(1, a)?; Ok(t) })?;
    let _: Table = f.call(1)?;
    let _ud = lua.create_userdata(Ud(1))?;
    lua.globals().set("ud", Ud(2))?;
    lua.load("return ud.v + ud:get(), tostring(ud)").eval::<(i32, String)>()?;
    let co = lua.create_thread(lua.load("local a = ...; coroutine.yield(a); return {a}").into_function()?)?;
    let _: i32 = co.resume(1)?;
    let _: Table = co.resume(())?;
    lua.scope(|scope| {
        let g = scope.create_function(|_, ()| Ok(1))?;
        let ud = scope.create_userdata(Ud(4))?;
        g.call::<i32>(())?;
        ud.call_method::<i32>("get", ())?;
        Ok(())
    })?;
    let v = lua.to_value(&vec![1, 2, 3])?;
    let _: Vec<i32> = lua.from_value(v)?;
    let r = lua.create_registry_value("x")?;
    lua.remove_registry_value(r)?;
    f.bind(1)?;
    lua.load("error('boom')").exec().err();
    lua.set_hook(HookTriggers::EVERY_LINE, |_, _| Ok(VmState::Continue))?;
    lua.remove_hook();
    let b = t.clone();
    b.to_string()?;
    Ok(())
}

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "memory_stress" => {
            let mut failures = 0u64; let mut okc = 0u64;
            for extra in (0..200_000usize).step_by(97) {
                let lua = Lua::new();
                ops(&lua).ok(); // warm up shared metatables
                lua.gc_collect()?; lua.gc_collect()?;
                let used = lua.used_memory();
                lua.set_memory_limit(used + extra / 8)?;
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ops(&lua)));
                match r {
                    Ok(Ok(())) => okc += 1,
                    Ok(Err(_)) => failures += 1,
                    Err(p) => {
                        let msg = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                        println!("PANIC at extra={}: {}", extra / 8, msg);
                        break;
                    }
                }
                lua.set_memory_limit(0)?;
            }
            println!("ok={okc} mem-errors={failures}");
        }
        "cold_stress" => {
            let mut res = (0u64, 0u64);
            for extra in (0..60_000usize).step_by(37) {
                let lua = Lua::new();
                let used = lua.used_memory();
                lua.set_memory_limit(used + extra)?;
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ops(&lua)));
                match r {
                    Ok(Ok(())) => res.0 += 1,
                    Ok(Err(_)) => res.1 += 1,
                    Err(p) => {
                        let msg = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                        println!("PANIC at extra={extra}: {msg}");
                        break;
                    }
                }
                lua.set_memory_limit(0)?;
            }
            println!("ok={} mem-errors={}", res.0, res.1);
        }
        "ref_oom" => {
            lua.set_memory_limit(lua.used_memory() + 512 * 1024)?;
            let mut v = Vec::new();
            loop {
                match lua.create_string("x") {
                    Ok(s) => v.push(s),
                    Err(e) => { println!("Err after {} refs: {}", v.len(), e.to_string().lines().next().unwrap()); break; }
                }
            }
        }
        "hook_control" => {
            let f = lua.load("local t = {1,2,3}\nlocal s = {name='alive'}\nlocal a = t[1]\nlocal b = s.name\nreturn a, b").into_function()?;
            let co = lua.create_thread(f)?;
            co.set_hook(HookTriggers::EVERY_LINE, |_, _| Ok(VmState::Yield))?;
            let mut last = None;
            while co.status() == mlua::ThreadStatus::Resumable { last = Some(co.resume::<(Option<i32>, Option<String>)>(()).map_err(|e| e.to_string())); }
            println!("no gc: {:?}", last);
        }
        "gc_control" => {
            let before = lua.used_memory();
            for _ in 0..20000 { let _ = lua.create_table()?; }
            lua.gc_collect()?;
            println!("control used memory {} -> {}", before, lua.used_memory());
        }
        "pretty" => {
            let v: Value = lua.load(r#"return {[""]=1, ["end"]=2}"#).eval()?;
            println!("{:#?}", v);
        }
        "path_min" => {
            let t = lua.create_table()?;
            t.raw_set(i64::MIN, "min")?;
            println!("{:?}", t.get_path::<String>("[-9223372036854775808]").map_err(|e| e.to_string()));
        }
        _ => panic!(),
    }
    Ok(())
}
