#![allow(unused)]
use mlua::prelude::*;
use mlua::{Nil, Value, Table, Function, Thread, UserData, AnyUserData, MultiValue};
use std::io::{Read, Seek, SeekFrom, Write};
struct H(Table); impl UserData for H {}
struct HF(Function); impl UserData for HF {}
fn main() -> LuaResult<()> {
    let a = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match a.as_str() {
        "nilkey" => { let t = lua.create_table()?; let r = t.set(Nil, 1); println!("{r:?}"); }
        "seek" => {
            let b = lua.create_buffer(b"hello")?; let mut c = b.cursor();
            println!("{:?}", c.seek(SeekFrom::Start(u64::MAX)));
            println!("{:?}", c.seek(SeekFrom::End(i64::MAX)));
            println!("{:?}", c.seek(SeekFrom::Current(2)));
            println!("{:?}", c.seek(SeekFrom::Current(i64::MAX)));
        }
        "readbytes" => {
            let b = lua.create_buffer(b"hello")?; let x = b.read_bytes::<4>(usize::MAX); println!("{x:?}");
        }
        "udtable_gc" => {
            let t = lua.create_table()?;
            lua.create_userdata(H(t))?;
            lua.gc_collect()?; lua.gc_collect()?; println!("ok after gc");
            let v: i32 = lua.load("return 1").eval()?; println!("{v}");
        }
        "udbomb" => {
            struct Bomb; impl Drop for Bomb { fn drop(&mut self) { panic!("boom") } } impl UserData for Bomb {}
            lua.create_userdata(Bomb)?;
            let r = lua.gc_collect(); println!("{r:?}");
        }
        "ro_insert" => {
            let t = lua.create_sequence_from([1,2,3])?; t.set_readonly(true);
            println!("{:?}", t.raw_insert(1, 5));
            println!("{:?}", t.raw_remove(1));
            println!("{:?}", t.clear());
            println!("{:?}", t.raw_set("a", 1));
            println!("{:?}", t.raw_seti(1, 1));
            println!("{:?}", t.raw_pop::<i32>());
            println!("{:?}", t.push(1));
            println!("{:?}", t.set_metatable(None));
        }
        "req3" => { let v: LuaResult<Value> = lua.load(std::path::Path::new("main.luau")).eval(); println!("{v:?}"); }
        "int_shift" => {
            lua.globals().set("fail", lua.create_function(|_,()| -> LuaResult<()> { Err(LuaError::runtime("fail")) })?)?;
            lua.set_interrupt(|lua| { let g: Option<Function> = lua.globals().get("G")?; if let Some(g) = g { println!("G()={:?}", g.call::<Value>(())); } Ok(mlua::VmState::Continue) });
            let r = lua.load("local a,b,c = 111,222,333\nG=function() return b end\nfor i=1,3 do pcall(fail) end\nfor i=1,3 do local z=i end\nreturn a+b+c").eval::<i32>(); println!("{r:?}");
        }
        "req1" => { let v: LuaResult<Value> = lua.load(std::path::Path::new("../m1.luau")).eval(); println!("{v:?}"); }
        "req2" => { let v: LuaResult<Value> = lua.load(r#"return require("../x")"#).set_name("@main").eval(); println!("{v:?}"); }
        "req" => {
            let v: LuaResult<Value> = lua.load(std::path::Path::new("main.luau")).eval(); println!("{v:?}");
        }
        "thread_status_new" => {
            let th: Thread = lua.load("coroutine.create(function() end)").eval()?; println!("{:?}", th.status());
            let f = lua.create_function(|_,()| Ok(()))?;
            th.resume::<()>(())?; println!("{:?}", th.status()); th.reset(f)?; println!("{:?}", th.status());
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
        "vec" => {
            let v: Value = lua.load("return vector.create(1,2,3)").eval()?; println!("{:?}", serde_json::to_string(&v));
            let a: [f32;3] = lua.unpack(v.clone())?; println!("{a:?}");
            let n: LuaResult<[String;3]> = lua.unpack(v.clone()); println!("{n:?}");
            let t: LuaResult<[Vec<i32>;3]> = lua.unpack(v.clone()); println!("{t:?}");
        }
        "sbx" => {
            lua.sandbox(true)?; let r = lua.load("x=1 return x").eval::<i32>(); println!("{r:?}");
            let g = lua.globals(); println!("{:?}", g.is_readonly());
            let f: Function = lua.load("return x").into_function()?;
            println!("{:?}", lua.set_globals(lua.create_table()?));
            lua.sandbox(false)?;
        }
        _ => panic!("unknown"),
    }
    Ok(())
}
