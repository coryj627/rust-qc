#![allow(unused)]
use mlua::prelude::*; use mlua::{Value,Nil,Table,Function,Thread};
use std::path::{Path,PathBuf}; use std::io::{Seek,SeekFrom,Read};
type R=LuaResult<()>;
fn setup()->PathBuf{let d=std::env::temp_dir().join("rq");let _=std::fs::remove_dir_all(&d);std::fs::create_dir_all(d.join("sub")).unwrap();std::fs::create_dir_all(d.join("lib")).unwrap();
 std::fs::write(d.join("main.luau"),"return require('./y')\n").unwrap();std::fs::write(d.join("y.luau"),"return 'Y'\n").unwrap();
 std::fs::write(d.join("lib/x.luau"),"return require('./y')\n").unwrap();std::fs::write(d.join("lib/y.luau"),"return 'libY'\n").unwrap();d}
fn req_file()->R{let d=setup();std::env::set_current_dir(&d).unwrap();let lua=Lua::new();
 println!("Path main.luau: {:?}",lua.load(Path::new("main.luau")).eval::<String>().map_err(|e|e.to_string().lines().next().unwrap().to_string()));
 println!("Path ./main.luau: {:?}",lua.load(PathBuf::from("./main.luau")).eval::<String>().map_err(|e|e.to_string().lines().next().unwrap().to_string()));
 println!("named @main: {:?}",lua.load("return require('./y')").set_name("@main").eval::<String>().map_err(|e|e.to_string().lines().next().unwrap().to_string()));Ok(())}
fn req_up()->R{let d=setup();std::env::set_current_dir(d.join("sub")).unwrap();let lua=Lua::new();
 println!("from sub: {:?}",lua.load("return require('../lib/x')").set_name("@./main").eval::<String>().map_err(|e|e.to_string().lines().next().unwrap().to_string()));
 println!("from sub @main: {:?}",lua.load("return require('../lib/x')").set_name("@main").eval::<String>().map_err(|e|e.to_string().lines().next().unwrap().to_string()));
 std::env::set_current_dir(&d).unwrap();let lua=Lua::new();println!("from parent: {:?}",lua.load("return require('./lib/x')").set_name("@main").eval::<String>().map_err(|e|e.to_string().lines().next().unwrap().to_string()));Ok(())}
fn vector()->R{#[derive(serde::Serialize)]struct Vector(String,String,String);#[derive(serde::Serialize)]struct Other(String,String,String);let lua=Lua::new();
 println!("{:?}",lua.to_value(&Vector("a".into(),"b".into(),"c".into())).map(|_|()).map_err(|e|e.to_string()));println!("{:?}",lua.to_value(&Other("a".into(),"b".into(),"c".into())).map(|_|()).map_err(|e|e.to_string()));Ok(())}
fn seek()->R{let lua=Lua::new();let b=lua.create_buffer(b"hello")?;let mut c=b.clone().cursor();
 let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||c.seek(SeekFrom::End(i64::MAX)).map_err(|e|e.to_string())));println!("End(MAX): {:?}",r.map_err(|_|"PANIC"));
 let mut c=b.clone().cursor();println!("Start(MAX): {:?}",c.seek(SeekFrom::Start(u64::MAX)).map_err(|e|e.to_string()));
 let mut c=b.clone().cursor();c.seek(SeekFrom::Start(1)).unwrap();let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||c.seek(SeekFrom::Current(i64::MAX)).map_err(|e|e.to_string())));println!("Current(MAX): {:?}",r.map_err(|_|"PANIC"));
 let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||b.read_bytes::<4>(usize::MAX)));println!("read_bytes(MAX): {:?}",r.map_err(|_|"PANIC"));
 let r=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||b.read_bytes::<4>(10)));println!("read_bytes(10): {:?}",r.map_err(|_|"PANIC"));Ok(())}
fn nilkey()->R{let lua=Lua::new();let t=lua.create_table()?;println!("{:?}",t.raw_set(Nil,1));Ok(())}
fn main(){let a=std::env::args().nth(1).unwrap();let r=match a.as_str(){"req_file"=>req_file(),"req_up"=>req_up(),"vector"=>vector(),"seek"=>seek(),"nilkey"=>nilkey(),_=>panic!()};println!("result: {:?}",r.map_err(|e|e.to_string()));}
