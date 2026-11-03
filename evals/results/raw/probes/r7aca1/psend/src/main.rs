use mlua::{Lua, Result, UserDataRef, UserDataRefMut, Function};
use std::cell::Cell;
use std::sync::mpsc;

fn assert_sync<T: Sync>(_: &T) {}

fn main() -> Result<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "ref_sync" => {
            let ud = lua.create_any_userdata(Cell::new(0u64))?;
            let r: UserDataRef<Cell<u64>> = ud.borrow()?;
            assert_sync(&r); // compiles: UserDataRef<Cell<_>> is Sync
            const N: u64 = 20_000_000;
            std::thread::scope(|s| {
                for _ in 0..4 {
                    s.spawn(|| { for _ in 0..N { r.set(std::hint::black_box(r.get()) + 1); } });
                }
            });
            println!("expected {}, got {}", 4 * N, r.get());
        }
        "refmut_sync" => {
            let ud = lua.create_any_userdata(Cell::new(0u64))?;
            let r: UserDataRefMut<Cell<u64>> = ud.borrow_mut()?;
            assert_sync(&r);
            const N: u64 = 20_000_000;
            std::thread::scope(|s| {
                for _ in 0..4 {
                    s.spawn(|| { for _ in 0..N { r.set(std::hint::black_box(r.get()) + 1); } });
                }
            });
            println!("expected {}, got {}", 4 * N, r.get());
        }
        "scope_window" => {
            // A scoped closure that is not Send, called from another thread through a Send Function handle.
            let (tx, rx) = mpsc::channel::<Function>();
            let (back_tx, back_rx) = mpsc::channel::<()>();
            let worker = std::thread::spawn(move || {
                let me = std::thread::current().id();
                let _ = me;
                for f in rx { let _ = f.call::<()>(()); let _ = back_tx.send(()); }
            });
            let main_id = std::thread::current().id();
            let mut foreign = 0u64; let mut total = 0u64;
            let hits = std::rc::Rc::new(Cell::new(0u64));
            for _ in 0..200_000 {
                total += 1;
                let h = hits.clone();
                lua.scope(|s| {
                    let f = s.create_function(move |_, ()| {
                        // `h` is an Rc: !Send. Touching it from another thread is a data race.
                        if std::thread::current().id() != main_id { h.set(h.get() + 1); }
                        Ok(())
                    })?;
                    tx.send(f).unwrap();
                    std::thread::yield_now();
                    Ok(())
                })?;
                back_rx.recv().unwrap();
                foreign = hits.get();
            }
            println!("scoped !Send closure ran on a foreign thread {foreign} times out of {total}");
            drop(tx);
            worker.join().unwrap();
        }
        "take_race" => {
            let (tx, rx) = mpsc::channel::<UserDataRef<u64>>();
            let (ack_tx, ack_rx) = mpsc::channel::<()>();
            let worker = std::thread::spawn(move || {
                for r in rx { let _ = ack_tx.send(()); drop(r); }
            });
            for i in 0..2_000_000u64 {
                let ud = lua.create_any_userdata(i)?;
                let r: UserDataRef<u64> = ud.borrow()?;
                tx.send(r).unwrap();
                ack_rx.recv().unwrap();
                // spin until the other thread has released the borrow
                loop {
                    match ud.take::<u64>() {
                        Ok(_) => break,
                        Err(mlua::Error::UserDataBorrowMutError) => continue,
                        Err(e) => panic!("{e}"),
                    }
                }
            }
            drop(tx);
            worker.join().unwrap();
            println!("no panic");
        }
        "shared_borrow" => {
            let ud = lua.create_any_userdata(5u64)?;
            let a = ud.borrow::<u64>()?;
            let b = ud.borrow::<u64>();
            println!("first {} second {:?}", *a, b.map(|r| *r));
            let lua2 = Lua::new();
            lua2.register_userdata_type::<u64>(|r| { r.add_method("m", |_, _, ()| Ok(1)); })?;
            let ud2 = lua2.create_any_userdata(5u64)?;
            let _hold = ud2.borrow::<u64>()?;
            lua2.globals().set("u", ud2.clone())?;
            println!("method while borrowed: {:?}", lua2.load("return u:m()").eval::<i32>().map_err(|e| e.to_string().lines().next().unwrap().to_string()));
        }
        _ => panic!(),
    }
    Ok(())
}
