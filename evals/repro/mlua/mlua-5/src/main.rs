// 686720c: Table::set_metatable reads a foreign table's index from this state's ref thread and
// passes whatever is in that slot to lua_setmetatable, which casts it to Table*.
use mlua::{Lua, Result};

fn main() -> Result<()> {
    let mode = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    let other = Lua::new();

    // Same creation order in both states, so `mt` in `other` and `s` in `lua` share a slot index.
    // A long Lua string's first 8 content bytes overlay Table.node when read as a Table.
    let t = lua.create_table()?;
    let byte = if mode == "foreign_0x42" { 0x42u8 } else { 0x41u8 };
    let s = lua.create_string([byte; 64])?;
    let _other_t = other.create_table()?;
    let mt = other.create_table()?;
    let _keep = &s;

    if mode == "same_state_control" {
        let local = lua.create_table()?;
        println!("r = {:?}", t.set_metatable(Some(local)));
        println!("get: {:?}", t.get::<mlua::Value>("missing"));
        return Ok(());
    }

    println!("calling set_metatable with a table from another Lua state");
    let r = t.set_metatable(Some(mt));
    println!("set_metatable returned: {r:?}");
    println!("get: {:?}", t.get::<mlua::Value>("missing"));
    Ok(())
}
