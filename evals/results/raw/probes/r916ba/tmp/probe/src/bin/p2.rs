use mlua::prelude::*;
use mlua::{Thread, Value, UserData, UserDataMethods, AnyUserData};

struct Scoped { alive: bool, name: String }
impl Drop for Scoped { fn drop(&mut self) { self.alive = false; self.name = "DROPPED".repeat(3); println!("Scoped dropped"); } }
impl UserData for Scoped {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("peek", |_, this, ()| { println!("peek: alive={} name={:?}", this.alive, this.name); Ok(this.alive) });
    }
}

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "scope_uaf" => {
            lua.scope(|scope| {
                let ud = scope.create_userdata(Scoped { alive: true, name: "hello world, a long-ish string".into() })?;
                lua.globals().set("ud", ud)?;
                lua.load("peek = ud.peek; print('in scope', peek(ud))").exec()
            })?;
            let r = lua.load("print(pcall(function() return ud:peek() end)); print('direct', pcall(peek, ud))").exec();
            println!("{:?}", r);
            for _ in 0..5 { let _v: Vec<String> = (0..100).map(|i| format!("junk{i}")).collect(); }
        }
        "scope_any_uaf" => {
            lua.scope(|scope| {
                let ud = scope.create_any_userdata(vec![1u8; 32], |reg| {
                    reg.add_method("len", |_, this: &Vec<u8>, ()| { Ok(this.len()) });
                    reg.add_method_mut("push", |_, this: &mut Vec<u8>, ()| { this.push(9); Ok(this.len()) });
                })?;
                lua.globals().set("ud", ud)?;
                lua.load("len = ud.len; push = ud.push").exec()
            })?;
            let r = lua.load("print('len', pcall(len, ud)); print('push', pcall(push, ud))").exec();
            println!("{:?}", r);
        }
        "tbc_lua_frame" => {
            let co: Thread = lua.load(r#"
              coroutine.create(function()
                local x <close> = setmetatable({}, {__close=function() print('close ran'); error('boom in close') end})
                local y = nil + 1
              end)
            "#).eval()?;
            let r: LuaResult<()> = co.resume(());
            println!("resume -> {:?}", r.map_err(|e| e.to_string()));
            println!("survived; status {:?}", co.status());
        }
        "tbc_yield_drop" => {
            let co: Thread = lua.load(r#"
              coroutine.create(function()
                local x <close> = setmetatable({}, {__close=function() print('close ran'); error(true) end})
                coroutine.yield(1)
              end)
            "#).eval()?;
            let r: LuaResult<i32> = co.resume(());
            println!("resume -> {:?}", r.map_err(|e| e.to_string()));
            let f = lua.create_function(|_, ()| Ok(()))?;
            println!("reset -> {:?}", co.reset(f).map_err(|e| e.to_string()));
        }
        "resume_main_cb" => {
            let main_th: Thread = lua.load("return (coroutine.running())").eval()?;
            let inner = lua.create_function(move |_, ()| {
                println!("main status {:?}", main_th.status());
                let r = main_th.resume::<()>(());
                println!("resume main -> {:?}", r.map_err(|e| e.to_string()));
                Ok(())
            })?;
            let co = lua.create_thread(inner)?;
            let outer = lua.create_function(move |_, (a, b): (i32, i32)| {
                let r: LuaResult<()> = co.resume(());
                println!("co.resume {:?}", r);
                Ok(a + b)
            })?;
            lua.globals().set("outer", outer)?;
            let r: LuaResult<i32> = lua.load("local p, q = 1, 2; local r = outer(3, 4); return r + p + q").eval();
            println!("{:?}", r);
        }
        "reset_main_cb" => {
            let main_th: Thread = lua.load("return (coroutine.running())").eval()?;
            let inner = lua.create_function(move |lua, ()| {
                let r = main_th.reset(lua.create_function(|_, ()| Ok(()))?);
                println!("reset main -> {:?}", r.map_err(|e| e.to_string()));
                Ok(())
            })?;
            let co = lua.create_thread(inner)?;
            let outer = lua.create_function(move |_, (a, b): (i32, i32)| {
                let r: LuaResult<()> = co.resume(());
                println!("co.resume {:?}", r);
                Ok(a + b)
            })?;
            lua.globals().set("outer", outer)?;
            let r: LuaResult<i32> = lua.load("local p, q = 1, 2; local r = outer(3, 4); return r + p + q").eval();
            println!("{:?}", r);
        }
        "mixed_empty" => {
            let t: Value = lua.load("return {}").eval()?;
            println!("default: {}", serde_json::to_string(&t.to_serializable()).unwrap());
            println!("mixed: {}", serde_json::to_string(&t.to_serializable().detect_mixed_tables(true)).unwrap());
            let t: Value = lua.load("return {[5]=1}").eval()?;
            println!("mixed sparse: {}", serde_json::to_string(&t.to_serializable().detect_mixed_tables(true)).unwrap());
        }
        "value_eq" => {
            let a = Value::Integer(i64::MAX); let b = Value::Number(9223372036854775807.0);
            println!("rust: {}", a == b);
            lua.globals().set("a", a)?; lua.globals().set("b", b)?;
            let r: bool = lua.load("return a == b").eval()?;
            println!("lua: {r}");
        }
        "ud_unused" => { let _: Option<AnyUserData> = None; }
        _ => panic!("unknown"),
    }
    Ok(())
}
