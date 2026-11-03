use mlua::prelude::*;
use std::cell::Cell;
use mlua::UserDataRef;
fn assert_sync<T: Sync>(_: &T) {}
fn main() -> LuaResult<()> {
    let lua = Lua::new();
    let ud = lua.create_any_userdata(Cell::new(0u64))?;
    let r: UserDataRef<Cell<u64>> = ud.borrow()?;
    assert_sync(&r);
    const N: u64 = 20_000_000;
    std::thread::scope(|s| {
        for _ in 0..2 {
            s.spawn(|| { for _ in 0..N { r.set(std::hint::black_box(r.get()) + 1); } });
        }
    });
    println!("expected {} got {}", 2 * N, r.get());
    // is_sync
    let ud2 = lua.create_any_userdata(5u64)?;
    let a = ud2.borrow::<u64>()?;
    println!("second shared borrow of Sync u64: {:?}", ud2.borrow::<u64>().map(|v| *v));
    drop(a);
    Ok(())
}
