use mlua::prelude::*;
use mlua::{Function, UserDataRef};
use std::cell::Cell;
use std::future::Future;
use std::task::{Context, Waker};

fn assert_sync<T: Sync>(_: &T) {}
async fn tokio_free_yield() {
    let mut done = false;
    std::future::poll_fn(|cx| { if done { std::task::Poll::Ready(()) } else { done = true; cx.waker().wake_by_ref(); std::task::Poll::Pending } }).await
}
fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "ud_ref_sync" => {
            // `Cell<u64>` is Send but not Sync
            lua.globals().set("ud", lua.create_any_userdata(Cell::new(0u64))?)?;
            let r: UserDataRef<Cell<u64>> = lua.globals().get("ud")?;
            assert_sync(&r); // compiles: UserDataRef<Cell<_>> is Sync
            const N: u64 = 20_000_000;
            std::thread::scope(|s| {
                for _ in 0..4 {
                    s.spawn(|| { for _ in 0..N { r.set(r.get() + 1); } });
                }
            });
            println!("expected {} got {}", 4 * N, r.get());
        }
        "cancel_runs_lua" => {
            let f: Function = lua.load(r#"return function()
                local v = coroutine.yield()
                ran_after_drop = tostring(v)
                error('from dropped future')
            end"#).eval()?;
            let mut fut = Box::pin(f.call_async::<()>(()));
            let w = Waker::noop();
            let mut cx = Context::from_waker(&w);
            println!("poll: {:?}", fut.as_mut().poll(&mut cx).is_pending());
            println!("before drop: {:?}", lua.globals().get::<Option<String>>("ran_after_drop")?);
            drop(fut);
            println!("after drop: {:?}", lua.globals().get::<Option<String>>("ran_after_drop")?);
        }
        "cancel_spin" => {
            // never-ending loop that only checks after the yield
            let f: Function = lua.load(r#"return function()
                coroutine.yield()
                while true do end
            end"#).eval()?;
            let mut fut = Box::pin(f.call_async::<()>(()));
            let w = Waker::noop();
            let mut cx = Context::from_waker(&w);
            println!("poll: {:?}", fut.as_mut().poll(&mut cx).is_pending());
            println!("dropping");
            drop(fut);
            println!("dropped");
        }
        "no_coroutine_global" => {
            lua.load("coroutine = nil").exec()?;
            println!("{:?}", lua.create_async_function(|_, ()| async { Ok(()) }).map(|_| "ok").map_err(|e| e.to_string()));
            let lua = Lua::new();
            lua.load("real = coroutine.yield; coroutine.yield = function(...) hijacked = true; return real(...) end").exec()?;
            let f = lua.create_async_function(|_, ()| async { tokio_free_yield().await; Ok(()) })?;
            let mut fut = Box::pin(f.call_async::<()>(()));
            let w = Waker::noop();
            let mut cx = Context::from_waker(&w);
            while fut.as_mut().poll(&mut cx).is_pending() {}
            println!("internal poller used script-provided yield: {:?}", lua.globals().get::<Option<bool>>("hijacked")?);
        }
        "pool_hook" => {
            let lua = Lua::new_with(LuaStdLib::ALL_SAFE, LuaOptions::new().thread_pool_size(4))?;
            let f: Function = lua.load("return function() local s=0 for i=1,1000 do s=s+i end return s end").eval()?;
            let w = Waker::noop();
            let mut cx = Context::from_waker(&w);
            // first call puts a thread in the pool
            { let mut fut = Box::pin(f.call_async::<i64>(())); println!("{:?}", fut.as_mut().poll(&mut cx).map(|r| r.is_ok())); }
            let c = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let c2 = c.clone();
            lua.set_global_hook(mlua::HookTriggers::new().every_nth_instruction(10), move |_, _| { c2.fetch_add(1, std::sync::atomic::Ordering::Relaxed); Ok(mlua::VmState::Continue) })?;
            { let mut fut = Box::pin(f.call_async::<i64>(())); println!("{:?}", fut.as_mut().poll(&mut cx).map(|r| r.is_ok())); }
            println!("hook fired on recycled thread: {}", c.load(std::sync::atomic::Ordering::Relaxed));
            let th = lua.create_thread(f.clone())?; th.resume::<i64>(())?;
            println!("hook fired after new thread: {}", c.load(std::sync::atomic::Ordering::Relaxed));
        }
        _ => panic!("unknown"),
    }
    Ok(())
}
