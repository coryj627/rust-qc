use mlua::prelude::*;
use mlua::{UserDataRef, Thread};
use std::cell::Cell;

fn assert_sync<T: Sync>() {}

async fn pingpong(lua: Lua, mut val: i32) -> LuaResult<()> {
    loop { val = lua.yield_with::<i32>(val).await? + 1; }
    #[allow(unreachable_code)]
    Ok(())
}

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "race" => {
            assert_sync::<UserDataRef<Cell<u64>>>();
            let ud = lua.create_any_userdata(Cell::new(0u64))?;
            let r: UserDataRef<Cell<u64>> = ud.borrow()?;
            const N: u64 = 5_000_000;
            std::thread::scope(|s| {
                for _ in 0..4 {
                    s.spawn(|| { for _ in 0..N { r.set(r.get() + 1); } });
                }
            });
            println!("expected {} got {}", 4 * N, r.get());
        }
        "yield_with_new" => {
            let co = lua.create_thread(lua.create_async_function(pingpong)?)?;
            println!("resume(1) = {:?}", co.resume::<i32>(1).map_err(|e| e.to_string()));
            let co2: Thread = co.clone();
            let outer = lua.create_function(move |_, v: i32| { co2.resume::<i32>(v) })?;
            println!("via callback = {:?}", outer.call::<i32>(2).map_err(|e| e.to_string()));
            println!("via callback = {:?}", outer.call::<i32>(3).map_err(|e| e.to_string()));
            println!("direct = {:?}", co.resume::<i32>(4).map_err(|e| e.to_string()));
        }
        "scope_window" => {
            use std::rc::Rc;
            use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
            use std::sync::Arc;
            let stop = Arc::new(AtomicBool::new(false));
            let foreign = Arc::new(AtomicU64::new(0));
            let lua2 = lua.clone();
            let stop2 = stop.clone();
            let t = std::thread::spawn(move || {
                let mut calls = 0u64; let mut destructed = 0u64;
                while !stop2.load(Ordering::Relaxed) {
                    if let Ok(Some(f)) = lua2.globals().get::<Option<mlua::Function>>("f") {
                        match f.call::<i32>(()) { Ok(_) => calls += 1, Err(_) => destructed += 1 }
                    }
                }
                (calls, destructed)
            });
            let main_id = std::thread::current().id();
            let mut iterations = 0;
            for _ in 0..200_000 {
                iterations += 1;
                let rc = Rc::new(1i32); // !Send data captured by the scoped callback
                let foreign2 = foreign.clone();
                lua.scope(|scope| {
                    let f = scope.create_function(move |_, ()| {
                        if std::thread::current().id() != main_id { foreign2.fetch_add(1, Ordering::Relaxed); }
                        Ok(*rc)
                    })?;
                    lua.globals().set("f", f)?;
                    Ok(())
                })?;
                if foreign.load(Ordering::Relaxed) > 0 { break; }
            }
            stop.store(true, Ordering::Relaxed);
            println!("iterations {iterations}, scoped !Send callback ran on other thread {} times; other thread (ok, err) = {:?}", foreign.load(Ordering::Relaxed), t.join().unwrap());
        }
        "yield_with_value" => {
            async fn gen(lua: Lua, _: ()) -> LuaResult<()> {
                loop {
                    let v: mlua::Value = lua.yield_with::<mlua::Value>(1).await?;
                    println!("generator received: {:?}", v);
                }
                #[allow(unreachable_code)]
                Ok(())
            }
            let co = lua.create_thread(lua.create_async_function(gen)?)?;
            println!("resume = {:?}", co.resume::<LuaMultiValue>(()).map_err(|e| e.to_string()));
            let co2: Thread = co.clone();
            let outer = lua.create_function(move |_, v: i32| { co2.resume::<LuaMultiValue>(v) })?;
            println!("via callback = {:?}", outer.call::<LuaMultiValue>(2).map_err(|e| e.to_string()));
            let bad = lua.create_function(|_, ()| Err::<(), _>(LuaError::runtime("x")))?;
            for i in 0..4 { println!("bad call {i} = {:?}", bad.call::<()>(()).map_err(|e| e.to_string().lines().next().unwrap().to_string())); }
            println!("done");
        }
        _ => panic!(),
    }
    Ok(())
}
