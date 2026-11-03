use mlua::{Lua, Result};
fn main() -> Result<()> {
    let lua = Lua::new();
    lua.load("setmetatable({}, {__gc = function() error('boom in gc') end})").exec()?;
    for i in 0..5_000_000u64 {
        match lua.create_table() { Ok(_) => {}, Err(e) => { println!("Err at {i}: {e}"); return Ok(()); } }
    }
    println!("finished without error");
    Ok(())
}
