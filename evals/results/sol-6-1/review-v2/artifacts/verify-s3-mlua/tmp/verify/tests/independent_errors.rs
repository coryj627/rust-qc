use mlua::{Lua,Result,Value};
#[test]
fn invalid_nil_raw_set_returns_error()->Result<()> {
    let lua=Lua::new();
    assert!(lua.create_table()?.raw_set(Value::Nil,1).is_err());
    Ok(())
}
#[test]
fn invalid_nan_set_returns_error()->Result<()> {
    let lua=Lua::new();
    assert!(lua.create_table()?.set(f64::NAN,1).is_err());
    Ok(())
}
#[test]
fn invalid_nil_set_returns_error()->Result<()> {
    let lua=Lua::new();
    assert!(lua.create_table()?.set(Value::Nil,1).is_err());
    Ok(())
}
#[test]
fn invalid_nan_raw_set_returns_error()->Result<()> {
    let lua=Lua::new();
    assert!(lua.create_table()?.raw_set(f64::NAN,1).is_err());
    Ok(())
}
#[test]
fn invalid_key_create_table_from_returns_error()->Result<()> {
    let lua=Lua::new();
    assert!(lua.create_table_from([(Value::Nil,1)]).is_err());
    Ok(())
}
#[test]
fn invalid_keys_with_memory_limit_return_errors()->Result<()> {
    let lua=Lua::new();
    lua.set_memory_limit(8*1024*1024)?;
    assert!(lua.create_table()?.raw_set(Value::Nil,1).is_err());
    assert!(lua.create_table()?.set(f64::NAN,1).is_err());
    assert!(lua.create_table_from([(Value::Nil,1)]).is_err());
    Ok(())
}
