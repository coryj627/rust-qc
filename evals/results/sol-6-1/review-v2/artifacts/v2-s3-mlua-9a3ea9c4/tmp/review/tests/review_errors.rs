use mlua::{Error, Lua, Result, Value};

#[test]
fn nil_table_key_returns_error() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_table()?;
    assert!(matches!(table.set(Value::Nil, 1), Err(Error::RuntimeError(_))));
    Ok(())
}

#[test]
fn nan_table_key_returns_error() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_table()?;
    assert!(matches!(table.raw_set(f64::NAN, 1), Err(Error::RuntimeError(_))));
    Ok(())
}

#[test]
fn nil_table_key_with_memory_limit_returns_error() -> Result<()> {
    let lua = Lua::new();
    lua.set_memory_limit(8 * 1024 * 1024)?;
    let table = lua.create_table()?;
    assert!(matches!(table.set(Value::Nil, 1), Err(Error::RuntimeError(_))));
    Ok(())
}
