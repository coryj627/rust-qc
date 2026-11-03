use mlua::prelude::*;
use mlua::{VmState, Table};
use std::io::{Seek, SeekFrom, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "interrupt_yield_gc" => {
            let count = Arc::new(AtomicU64::new(0));
            lua.set_interrupt(move |_| {
                if count.fetch_add(1, Ordering::Relaxed) % 2 == 0 { return Ok(VmState::Yield); }
                Ok(VmState::Continue)
            });
            let f = lua.load(r#"
                local t = {1, 2, 3}
                local s = {name = "alive"}
                local b = 0
                for i, x in ipairs(t) do b += x end
                for i = 1, 5 do b += #s.name end
                return b, s.name, t[1]
            "#).into_function()?;
            let co = lua.create_thread(f)?;
            let mut n = 0;
            while co.status() == mlua::ThreadStatus::Resumable {
                n += 1;
                let r = co.resume::<LuaMultiValue>(());
                if let Err(e) = &r { println!("resume #{n}: {}", e.to_string().lines().next().unwrap()); }
                else if co.status() != mlua::ThreadStatus::Resumable { println!("resume #{n}: {:?}", r); }
                lua.gc_collect()?; lua.gc_collect()?;
                if n > 200 { break; }
            }
            println!("final {:?} after {n}", co.status());
        }
        "seek" => {
            let b = lua.create_buffer(b"hello")?;
            let mut c = b.clone().cursor();
            println!("{:?}", c.seek(SeekFrom::End(-2)));
            let mut s = [0u8; 8]; println!("{:?}", c.read(&mut s));
            println!("{:?}", c.write(b"x"));
            println!("start max {:?}", c.seek(SeekFrom::Start(u64::MAX)).map_err(|e| e.to_string()));
            println!("end max {:?}", c.seek(SeekFrom::End(i64::MAX)).map_err(|e| e.to_string()));
        }
        "readonly" => {
            let t: Table = lua.load("return {1,2,3}").eval()?;
            t.set_readonly(true);
            println!("raw_set {:?}", t.raw_set(1, 9).map_err(|e| e.to_string()));
            println!("raw_insert {:?}", t.raw_insert(1, 9).map_err(|e| e.to_string().lines().next().unwrap().to_string()));
            println!("raw_remove {:?}", t.raw_remove(1).map_err(|e| e.to_string().lines().next().unwrap().to_string()));
            println!("raw_remove str {:?}", t.raw_remove("a").map_err(|e| e.to_string().lines().next().unwrap().to_string()));
            println!("push {:?}", t.push(1).map_err(|e| e.to_string().lines().next().unwrap().to_string()));
            println!("pop {:?}", t.pop::<i32>().map_err(|e| e.to_string().lines().next().unwrap().to_string()));
            println!("clear {:?}", t.clear().map_err(|e| e.to_string()));
            println!("set_mt {:?}", t.set_metatable(None).map_err(|e| e.to_string()));
            println!("raw_set nil key: {:?}", lua.create_table()?.raw_set(mlua::Value::Nil, 1).map_err(|e| e.to_string()));
        }
        "require_file" => {
            std::env::set_current_dir("/work/tmp")?;
            let r = lua.load(std::path::Path::new("req/main.luau")).eval::<String>();
            println!("load(req/main.luau) -> {:?}", r.map_err(|e| e.to_string().lines().take(2).collect::<Vec<_>>().join(" | ")));
            let r = lua.load(std::path::PathBuf::from("/work/tmp/req/main.luau")).eval::<String>();
            println!("load(abs main.luau) -> {:?}", r.map_err(|e| e.to_string().lines().take(2).collect::<Vec<_>>().join(" | ")));
            let r = lua.load(r#"return require("./req/mod")"#).set_name("@req/fake").eval::<String>();
            println!("name @req/fake -> {:?}", r.map_err(|e| e.to_string().lines().take(2).collect::<Vec<_>>().join(" | ")));
            let r = lua.load(r#"return require("./mod")"#).set_name("@req/main").eval::<String>();
            println!("name @req/main (no ext) -> {:?}", r.map_err(|e| e.to_string().lines().take(2).collect::<Vec<_>>().join(" | ")));
            let r = lua.load(r#"return require("./req/mod")"#).eval::<String>();
            println!("rs caller -> {:?}", r.map_err(|e| e.to_string().lines().take(2).collect::<Vec<_>>().join(" | ")));
        }
        "vector_name" => {
            #[derive(serde::Serialize)]
            struct Vector(String, String, String);
            #[derive(serde::Serialize)]
            struct Other(String, String, String);
            println!("Other -> {:?}", lua.to_value(&Other("a".into(), "b".into(), "c".into())).map(|v| v.type_name()).map_err(|e| e.to_string()));
            println!("Vector -> {:?}", lua.to_value(&Vector("a".into(), "b".into(), "c".into())).map(|v| v.type_name()).map_err(|e| e.to_string()));
        }
        _ => panic!(),
    }
    Ok(())
}
