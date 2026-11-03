// abe1c54: Lua::create_thread / Thread::reset with a Function from another Lua state.
use mlua::{Lua, Result};

fn main() -> Result<()> {
    let mode = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    let other = Lua::new();

    // Same creation order, so the foreign function has the same slot index as `local`.
    let local = lua.load("return 'LOCAL function ran'").into_function()?;
    let foreign = other.load("return 'FOREIGN function ran'").into_function()?;

    match mode.as_str() {
        "create_thread" => {
            let th = lua.create_thread(foreign)?;
            println!("resume: {:?}", th.resume::<String>(()));
        }
        "reset" => {
            let th = lua.create_thread(local.clone())?;
            let _: String = th.resume(())?;
            th.reset(foreign)?;
            println!("resume: {:?}", th.resume::<String>(()));
        }
        "out_of_range" => {
            // Push the foreign function's slot past the top of this state's ref stack.
            let mut keep = Vec::new();
            for _ in 0..64 {
                keep.push(other.create_table()?);
            }
            let far = other.load("return 'FOREIGN function ran'").into_function()?;
            let th = lua.create_thread(far)?;
            println!("resume: {:?}", th.resume::<String>(()));
        }
        _ => panic!("mode"),
    }
    let _ = local;
    Ok(())
}
