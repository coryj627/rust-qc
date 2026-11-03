use mlua::{Lua, LuaSerdeExt, Result, Table, Value, Nil};
use std::io::{Seek, SeekFrom, Read, Write};
fn main() -> Result<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "seek_end" => { let b = lua.create_buffer(b"abc")?; let mut c = b.cursor(); println!("{:?}", c.seek(SeekFrom::End(i64::MAX))); }
        "seek_cur" => { let b = lua.create_buffer(b"abc")?; let mut c = b.cursor(); c.seek(SeekFrom::Start(2)).unwrap(); println!("{:?}", c.seek(SeekFrom::Current(i64::MAX))); }
        "seek_start" => { let b = lua.create_buffer(b"abc")?; let mut c = b.cursor(); println!("{:?}", c.seek(SeekFrom::Start(u64::MAX))); println!("{:?}", c.seek(SeekFrom::Start(1<<63))); }
        "cursor_rw" => { let b = lua.create_buffer(b"abc")?; let mut c = b.clone().cursor(); let mut s = [0u8;2]; println!("{:?}", c.read(&mut s)); println!("{:?}", c.write(b"zzzz")); println!("{:?}", b.to_vec()); }
        "read_oob" => { let b = lua.create_buffer(b"abc")?; println!("{:?}", b.read_bytes::<2>(usize::MAX)); }
        "ro_insert" => {
            let t: Table = lua.load("return {1,2}").eval()?;
            t.set_readonly(true);
            println!("raw_set {:?}", t.raw_set(5, 1).map_err(|e| e.to_string().chars().take(60).collect::<String>()));
            println!("raw_insert {:?}", t.raw_insert(1, 9).map_err(|e| e.to_string().chars().take(60).collect::<String>()));
            println!("raw_remove {:?}", t.raw_remove(1).map_err(|e| e.to_string().chars().take(60).collect::<String>()));
            println!("clear {:?}", t.clear().map_err(|e| e.to_string().chars().take(60).collect::<String>()));
            println!("raw_len {}", t.raw_len());
        }
        "nil_key" => { let t = lua.create_table()?; println!("{:?}", t.set(Nil, 1)); }
        "vec_name" => {
            #[derive(serde::Serialize)] struct Vector(String, String, String);
            println!("{:?}", lua.to_value(&Vector("a".into(),"b".into(),"c".into())).map_err(|e| e.to_string()));
            #[derive(serde::Serialize)] struct Foo(String, String, String);
            println!("{:?}", lua.to_value(&Foo("a".into(),"b".into(),"c".into())).map(|v| v.type_name()));
        }
        "reset_normal" => {
            lua.globals().set("reset", lua.create_function(|lua, th: mlua::Thread| {
                println!("status {:?}", th.status());
                println!("reset {:?}", th.reset(lua.create_function(|_, ()| Ok(1))?));
                Ok(())
            })?)?;
            println!("{:?}", lua.load(r#"
                local co1
                co1 = coroutine.create(function()
                    local co2 = coroutine.create(function() reset(co1) end)
                    print(coroutine.resume(co2))
                    return 1
                end)
                return coroutine.resume(co1)
            "#).eval::<mlua::MultiValue>().map_err(|e| e.to_string()));
        }
        "sandbox" => {
            lua.sandbox(true)?;
            println!("{:?}", lua.load("x = 1; return x").eval::<i32>());
            let v: Value = lua.globals().get("x")?; println!("x {:?}", v);
            lua.sandbox(false)?;
            let v: Value = lua.globals().get("x")?; println!("x {:?}", v);
            lua.load("y = 1").exec()?;
        }
        "drop_handle_in_gc" => {
            struct H(mlua::Function, Table);
            let f = lua.create_function(|_, ()| Ok(()))?;
            for _ in 0..1000 {
                let ud = lua.create_any_userdata(H(f.clone(), lua.create_table()?))?;
                drop(ud);
            }
            lua.gc_collect()?; lua.gc_collect()?;
            lua.load("for i=1,100000 do local t = {i} end").exec()?;
            lua.gc_collect()?;
            let v: Vec<Table> = (0..100).map(|_| lua.create_table().unwrap()).collect();
            println!("ok {}", v.len());
        }
        _ => panic!(),
    }
    Ok(())
}
