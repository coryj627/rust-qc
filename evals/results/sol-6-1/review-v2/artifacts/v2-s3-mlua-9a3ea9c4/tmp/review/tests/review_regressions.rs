use mlua::{Lua, LuaSerdeExt, Value, DeserializeOptions, ObjectLike, Result};

#[test]
fn table_equality_requires_same_length() -> Result<()> {
    let lua = Lua::new();
    let empty = lua.create_table()?;
    let one = lua.create_sequence_from([1])?;
    assert_ne!(empty, [1]);
    assert_ne!(one, [1, 2]);
    Ok(())
}

#[test]
fn clear_preserves_stack() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_sequence_from([1, 2])?;
    unsafe {
        lua.exec_raw::<()>((), |state| {
            let before = mlua::ffi::lua_gettop(state);
            table.clear().unwrap();
            let after = mlua::ffi::lua_gettop(state);
            assert_eq!(after, before, "Table::clear must leave the stack unchanged");
        })?;
    }
    Ok(())
}

#[test]
fn relaxed_serde_preserves_null_array_elements() -> Result<()> {
    let lua = Lua::new();
    let value = lua.to_value(&vec![Some(1), None, Some(2)])?;
    let actual: Vec<Option<i64>> = lua.from_value_with(value, DeserializeOptions::new().deny_unsupported_types(false))?;
    assert_eq!(actual, vec![Some(1), None, Some(2)]);
    Ok(())
}

#[test]
fn relaxed_serialization_preserves_null_fields() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_table()?;
    table.set("x", Value::NULL)?;
    table.set("y", 1)?;
    let value = Value::Table(table);
    let actual = serde_json::to_value(value.to_serializable().deny_unsupported_types(false)).unwrap();
    assert_eq!(actual, serde_json::json!({"x": null, "y": 1}));
    Ok(())
}

#[test]
fn lookup_accepts_min_integer_key() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_table()?;
    table.set(i64::MIN, "minimum")?;
    assert_eq!(table.get_path::<String>("[-9223372036854775808]")?, "minimum");
    Ok(())
}

#[test]
fn table_equality_rejects_longer_slice() -> Result<()> {
    let lua = Lua::new();
    let one = lua.create_sequence_from([1])?;
    assert_ne!(one, [1, 2]);
    Ok(())
}

#[test]
fn clear_does_not_retain_table_after_drop() -> Result<()> {
    let lua = Lua::new();
    let weak: mlua::Table = lua.load("setmetatable({}, {__mode='v'})").eval()?;
    let control = lua.create_table()?;
    weak.set(1, &control)?;
    drop(control);
    lua.gc_collect()?;
    lua.gc_collect()?;
    assert_eq!(weak.get::<Value>(1)?, Value::Nil);
    let table = lua.create_table()?;
    weak.set(1, &table)?;
    table.clear()?;
    drop(table);
    lua.gc_collect()?;
    lua.gc_collect()?;
    assert_eq!(weak.get::<Value>(1)?, Value::Nil);
    Ok(())
}

#[test]
#[cfg(any(feature="lua54", feature="lua53", feature="lua52"))]
fn numeric_equality_matches_lua() -> Result<()> {
    let lua = Lua::new();
    let integer = Value::Integer(9007199254740993);
    let number = Value::Number(9007199254740992.0);
    lua.globals().set("integer", integer.clone())?;
    lua.globals().set("number", number.clone())?;
    let actual_lua: bool = lua.load("integer == number").eval()?;
    assert!(!actual_lua);
    assert_eq!(integer.equals(&number)?, actual_lua);
    Ok(())
}

#[test]
fn relaxed_serde_skips_trailing_unsupported_for_tuple() -> Result<()> {
    let lua = Lua::new();
    let value = lua.load("{1, 2, function() end}").eval()?;
    let actual: (i64, i64) = lua.from_value_with(value, DeserializeOptions::new().deny_unsupported_types(false))?;
    assert_eq!(actual, (1, 2));
    Ok(())
}
