use mlua::prelude::*;
fn main(){let lua=Lua::new();lua.load("setmetatable({}, {__gc=function() error('boom') end})").exec().unwrap();
 for i in 0..2000000{let t=lua.create_table();if let Err(e)=t{println!("err at {i}: {e}");return}}println!("survived");
 println!("{:?}",lua.gc_collect());}
