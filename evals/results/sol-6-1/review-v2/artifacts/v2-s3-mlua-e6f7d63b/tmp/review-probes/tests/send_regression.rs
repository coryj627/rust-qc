#![cfg(feature = "send")]
use mlua::{Lua, Result};

#[test]
fn sync_userdata_allows_two_shared_borrows() -> Result<()> {
    let lua = Lua::new();
    let userdata = lua.create_any_userdata(String::from("shared"))?;
    let _first = userdata.borrow::<String>()?;
    let _second = userdata.borrow::<String>()?;
    Ok(())
}
