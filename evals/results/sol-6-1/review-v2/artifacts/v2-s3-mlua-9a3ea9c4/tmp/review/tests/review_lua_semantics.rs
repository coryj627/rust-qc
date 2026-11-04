use mlua::{Lua, Result};
#[test]
fn empty_pop_matches_lua_table_remove() -> Result<()> {
    let lua = Lua::new();
    let lua_result: String = lua.load("local t = {[0]='metadata'}; return table.remove(t)").eval()?;
    let table = lua.create_table()?;
    table.set(0, "metadata")?;
    let rust_result: String = table.raw_pop()?;
    assert_eq!(lua_result, rust_result);
    assert_eq!(rust_result, "metadata");
    Ok(())
}
