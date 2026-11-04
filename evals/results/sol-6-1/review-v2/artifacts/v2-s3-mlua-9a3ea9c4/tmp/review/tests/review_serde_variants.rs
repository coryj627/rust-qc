use mlua::{Lua, LuaSerdeExt, Result, SerializeOptions, Value};
use serde::Serialize;

#[derive(Serialize)]
enum E { Tuple(i32, Option<i32>, i32) }

#[test]
fn nil_tuple_variant_field_does_not_shift_later_fields() -> Result<()> {
    let lua = Lua::new();
    let options = SerializeOptions::new().serialize_none_to_null(false);
    let value = lua.to_value_with(&E::Tuple(1, None, 2), options)?;
    let inner = value.as_table().unwrap().get::<mlua::Table>("Tuple")?;
    assert_eq!(inner.get::<Value>(2)?, Value::Nil);
    assert_eq!(inner.get::<i32>(3)?, 2);
    Ok(())
}

#[test]
fn plain_tuple_retains_nil_position_control() -> Result<()> {
    let lua = Lua::new();
    let options = SerializeOptions::new().serialize_none_to_null(false);
    let value = lua.to_value_with(&(1, None::<i32>, 2), options)?;
    let inner = value.as_table().unwrap();
    assert_eq!(inner.get::<Value>(2)?, Value::Nil);
    assert_eq!(inner.get::<i32>(3)?, 2);
    Ok(())
}
