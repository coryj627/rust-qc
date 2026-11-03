use mlua::prelude::*;
use mlua::Value;
use serde::Serialize;

#[derive(Serialize)]
struct Vector(String, String, String);

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "vector_name" => {
            let v = Vector("a".into(), "b".into(), "c".into());
            println!("{:?}", lua.to_value(&v).map_err(|e| e.to_string()));
        }
        "require_parent" => {
            let base = std::env::temp_dir().join("reqprobe");
            std::fs::create_dir_all(base.join("sub")).unwrap();
            std::fs::create_dir_all(base.join("lib")).unwrap();
            std::fs::write(base.join("lib/x.luau"), "return require('./y')").unwrap();
            std::fs::write(base.join("lib/y.luau"), "return 'y-value'").unwrap();
            std::env::set_current_dir(base.join("sub")).unwrap();
            let r: LuaResult<Value> = lua.load("return require('../lib/x')").set_name("@./main.luau").eval();
            println!("{:?}", r.map_err(|e| e.to_string()));
        }
        "require_parent2" => {
            let base = std::env::temp_dir().join("reqprobe2");
            std::fs::create_dir_all(base.join("sub")).unwrap();
            std::fs::create_dir_all(base.join("lib")).unwrap();
            std::fs::write(base.join("sub/main.luau"), "return require('../lib/x')").unwrap();
            std::fs::write(base.join("lib/x.luau"), "return require('./y')").unwrap();
            std::fs::write(base.join("lib/y.luau"), "return 'y-value'").unwrap();
            std::env::set_current_dir(base.join("sub")).unwrap();
            let r: LuaResult<Value> = lua.load(std::path::PathBuf::from("./main.luau")).eval();
            println!("from cwd sub, ./main.luau: {:?}", r.map_err(|e| e.to_string()));
            // Same tree, run from the parent directory (chunk stays inside cwd): works?
            std::env::set_current_dir(&base).unwrap();
            let lua2 = Lua::new();
            let r: LuaResult<Value> = lua2.load(std::path::PathBuf::from("./sub/main.luau")).eval();
            println!("from cwd base, ./sub/main.luau: {:?}", r.map_err(|e| e.to_string()));
        }
        "require_parent3" => {
            let base = std::env::temp_dir().join("reqprobe3");
            std::fs::create_dir_all(base.join("sub")).unwrap();
            std::fs::create_dir_all(base.join("lib")).unwrap();
            std::fs::write(base.join("sub/main.luau"), "").unwrap();
            std::fs::write(base.join("lib/x.luau"), "return require('./y')").unwrap();
            std::fs::write(base.join("lib/y.luau"), "return 'y-value'").unwrap();
            std::env::set_current_dir(base.join("sub")).unwrap();
            let r: LuaResult<Value> = lua.load("return require('../lib/x')").set_name("@./main").eval();
            println!("cwd sub, chunk @./main: {:?}", r.map_err(|e| e.to_string()));
            std::fs::write(base.join("lib/z.luau"), "return debug.info(1, 's')").unwrap();
            let lua2 = Lua::new();
            let r: LuaResult<Value> = lua2.load("return require('../lib/z')").set_name("@./main").eval();
            println!("chunk name of ../lib/z: {:?}", r.map_err(|e| e.to_string()));
            std::env::set_current_dir(&base).unwrap();
            let lua3 = Lua::new();
            let r: LuaResult<Value> = lua3.load("return require('../lib/x')").set_name("@./sub/main").eval();
            println!("cwd base, chunk @./sub/main: {:?}", r.map_err(|e| e.to_string()));
        }
        "cursor_seek" => {
            use std::io::{Seek, SeekFrom};
            let b = lua.create_buffer(b"hello")?;
            let mut c = b.cursor();
            println!("Start(u64::MAX): {:?}", c.seek(SeekFrom::Start(u64::MAX)).map_err(|e| e.to_string()));
            println!("End(i64::MAX): {:?}", c.seek(SeekFrom::End(i64::MAX)).map_err(|e| e.to_string()));
        }
        "nil_key" => {
            let t = lua.create_table()?;
            println!("{:?}", t.set(mlua::Nil, 1).map_err(|e| e.to_string()));
        }
        "readonly_remove" => {
            let t: LuaTable = lua.load("{1,2,3}").eval()?;
            t.set_readonly(true);
            println!("raw_insert {:?}", t.raw_insert(1, 5).map_err(|e| e.to_string()));
            println!("raw_remove {:?}", t.raw_remove(1).map_err(|e| e.to_string()));
            println!("clear {:?}", t.clear().map_err(|e| e.to_string()));
        }
        _ => panic!("unknown"),
    }
    Ok(())
}
