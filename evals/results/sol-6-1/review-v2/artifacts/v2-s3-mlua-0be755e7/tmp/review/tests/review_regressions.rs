use mlua::{Lua, LuaSerdeExt, Result, Table, Value};

#[test]
fn clear_does_not_retain_table() -> Result<()> {
    let lua = Lua::new();
    let weak: Table = lua.load("setmetatable({}, {__mode = 'v'})").eval()?;
    let control = lua.create_table()?;
    weak.set("control", &control)?;
    drop(control);
    let cleared = lua.create_table()?;
    weak.set("cleared", &cleared)?;
    cleared.clear()?;
    drop(cleared);
    lua.gc_collect()?;
    lua.gc_collect()?;
    assert_eq!(weak.get::<Value>("control")?, Value::Nil);
    assert_eq!(weak.get::<Value>("cleared")?, Value::Nil);
    Ok(())
}

#[test]
fn table_equality_checks_sequence_length() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_sequence_from([1, 2])?;
    assert_ne!(table, [1, 2, 3]);
    Ok(())
}

#[test]
fn relaxed_deserialization_preserves_null() -> Result<()> {
    let lua = Lua::new();
    let value = lua.to_value_with(&serde_json::json!({"present": null, "items": [null, 1]}), mlua::SerializeOptions::new().detect_serde_json_arbitrary_precision(true))?;
    let options = mlua::DeserializeOptions::new().deny_unsupported_types(false);
    let actual: serde_json::Value = lua.from_value_with(value, options)?;
    assert_eq!(actual, serde_json::json!({"present": null, "items": [null, 1]}));
    Ok(())
}

#[test]
fn hashset_numeric_roundtrip() -> Result<()> {
    let lua = Lua::new();
    let expected = std::collections::HashSet::from([1, 2, 3]);
    let actual = lua.convert::<std::collections::HashSet<i32>>(expected.clone())?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn btreeset_numeric_roundtrip() -> Result<()> {
    let lua = Lua::new();
    let expected = std::collections::BTreeSet::from([1, 2, 3]);
    let actual = lua.convert::<std::collections::BTreeSet<i32>>(expected.clone())?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
#[cfg(feature = "luau")]
fn buffer_seek_overflow_is_error() -> Result<()> {
    use std::io::{Seek, SeekFrom};
    let lua = Lua::new();
    let mut cursor = lua.create_buffer(b"abc")?.cursor();
    cursor.seek(SeekFrom::Start(1)).unwrap();
    assert!(cursor.seek(SeekFrom::Current(i64::MAX)).is_err());
    Ok(())
}

#[test]
fn userdata_equality_checks_first_metamethod() -> Result<()> {
    struct A;
    struct B;
    impl mlua::UserData for A {
        fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
            methods.add_meta_function(mlua::MetaMethod::Eq, |_, (_a, _b): (Value, Value)| Ok(true));
        }
    }
    impl mlua::UserData for B {}
    let lua = Lua::new();
    let a = Value::UserData(lua.create_userdata(A)?);
    let b = Value::UserData(lua.create_userdata(B)?);
    lua.globals().set("a", &a)?;
    lua.globals().set("b", &b)?;
    #[cfg(any(feature = "lua54", feature = "lua53"))]
    assert!(lua.load("a == b").eval::<bool>()?);
    assert!(a.equals(&b)?);
    Ok(())
}

#[test]
fn metatable_iterator_skips_restricted_entries_before_conversion() -> Result<()> {
    struct A;
    impl mlua::UserData for A {}
    let lua = Lua::new();
    let ud = lua.create_userdata(A)?;
    let mt = ud.metatable()?;
    let values = mt.pairs::<Value>().collect::<Result<Vec<_>>>()?;
    assert!(values.iter().all(|(_, value)| matches!(value, Value::String(_))));
    let entries = mt.pairs::<String>().collect::<Result<Vec<_>>>()?;
    assert!(entries.iter().all(|(k, _)| k != "__gc" && k != "__metatable"));
    Ok(())
}

#[test]
#[cfg(feature = "send")]
fn sync_userdata_allows_two_read_borrows() -> Result<()> {
    #[derive(Debug)]
    struct A;
    impl mlua::UserData for A {}
    let lua = Lua::new();
    let ud = lua.create_userdata(A)?;
    let _first = ud.borrow::<A>()?;
    let _second = ud.borrow::<A>()?;
    Ok(())
}

#[test]
fn relaxed_serialization_preserves_null() -> Result<()> {
    let lua = Lua::new();
    let original = serde_json::json!({"present": null, "items": [null, 1]});
    let value = lua.to_value_with(&original, mlua::SerializeOptions::new().detect_serde_json_arbitrary_precision(true))?;
    let actual = serde_json::to_value(value.to_serializable().deny_unsupported_types(false)).unwrap();
    assert_eq!(actual, original);
    Ok(())
}

#[test]
fn empty_table_does_not_equal_nonempty_slice() -> Result<()> {
    let lua = Lua::new();
    let empty = lua.create_table()?;
    assert_ne!(empty, [1]);
    Ok(())
}
