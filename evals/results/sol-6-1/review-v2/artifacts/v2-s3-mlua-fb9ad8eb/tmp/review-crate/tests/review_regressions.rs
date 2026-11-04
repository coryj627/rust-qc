use std::collections::{BTreeSet, HashSet};

use mlua::{DeserializeOptions, Lua, LuaSerdeExt, ObjectLike, Result, Value};

#[test]
fn clear_does_not_retain_table() -> Result<()> {
    let lua = Lua::new();
    let weak: mlua::Table = lua.load("setmetatable({}, {__mode = 'v'})").eval()?;
    {
        let table = lua.create_table()?;
        weak.set(1, &table)?;
        table.clear()?;
    }
    lua.gc_collect()?;
    lua.gc_collect()?;
    assert_eq!(weak.get::<Value>(1)?, Value::Nil);
    Ok(())
}

#[test]
fn table_equality_checks_full_slice_length() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_sequence_from([1, 2])?;
    let empty = lua.create_table()?;
    let extra_slice_matches = table == [1, 2, 3];
    let empty_matches_nonempty = empty == [1];
    assert!(
        !extra_slice_matches && !empty_matches_nonempty,
        "extra_slice_matches={extra_slice_matches}, empty_matches_nonempty={empty_matches_nonempty}"
    );
    Ok(())
}

#[test]
fn optional_null_values_survive_skipping_unsupported_types() -> Result<()> {
    let lua = Lua::new();
    let original = vec![Some(1), None, Some(3)];
    let value = lua.to_value(&original)?;
    let actual: Vec<Option<i32>> = lua.from_value_with(
        value,
        DeserializeOptions::new().deny_unsupported_types(false),
    )?;
    assert_eq!(actual, original);
    Ok(())
}

#[test]
fn null_map_entries_survive_serializable_filter() -> Result<()> {
    let lua = Lua::new();
    let value = lua.to_value(&serde_json::json!({"present": true, "absent": null}))?;
    let actual = serde_json::to_value(value.to_serializable().deny_unsupported_types(false)).unwrap();
    assert_eq!(actual, serde_json::json!({"present": true, "absent": null}));
    Ok(())
}

#[test]
fn integer_hashset_roundtrips() -> Result<()> {
    let lua = Lua::new();
    let original = HashSet::from([1, 2, 3]);
    let value = lua.pack(original.clone())?;
    let actual: HashSet<i32> = lua.unpack(value)?;
    assert_eq!(actual, original);
    Ok(())
}

#[test]
fn integer_btreeset_roundtrips() -> Result<()> {
    let lua = Lua::new();
    let original = BTreeSet::from([1, 2, 3]);
    let value = lua.pack(original.clone())?;
    let actual: BTreeSet<i32> = lua.unpack(value)?;
    assert_eq!(actual, original);
    Ok(())
}

#[test]
fn path_accepts_minimum_integer_key() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_table()?;
    table.set(i64::MIN, "minimum")?;
    assert_eq!(table.get_path::<String>("[-9223372036854775808]")?, "minimum");
    Ok(())
}

#[test]
fn invalid_nil_key_returns_error() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_table()?;
    assert!(table.raw_set(Value::Nil, 1).is_err());
    Ok(())
}

#[test]
fn nil_key_returns_error_when_memory_limit_enabled() -> Result<()> {
    let lua = Lua::new();
    lua.set_memory_limit(1024 * 1024)?;
    let table = lua.create_table()?;
    assert!(table.raw_set(Value::Nil, 1).is_err());
    Ok(())
}

#[test]
fn invalid_nan_key_returns_error() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_table()?;
    assert!(table.set(f64::NAN, 1).is_err());
    Ok(())
}

#[cfg(feature = "send")]
#[test]
fn sync_userdata_allows_two_shared_borrows() -> Result<()> {
    let lua = Lua::new();
    let ud = lua.create_any_userdata(String::from("shared"))?;
    let first = ud.borrow::<String>()?;
    let second = ud.borrow::<String>()?;
    assert_eq!(*first, *second);
    Ok(())
}

#[cfg(feature = "luau")]
#[test]
fn vector_to_short_tuple_rejects_extra_components() -> Result<()> {
    let lua = Lua::new();
    let value = Value::Vector(mlua::Vector::new(1.0, 2.0, 3.0));
    let actual = lua.from_value::<(f32, f32)>(value);
    assert!(actual.is_err(), "extra vector components were silently discarded: {actual:?}");
    Ok(())
}

#[cfg(feature = "luau")]
#[test]
fn buffer_seek_out_of_range_returns_error() -> Result<()> {
    use std::io::{Seek, SeekFrom};
    let lua = Lua::new();
    let mut cursor = lua.create_buffer(b"test")?.cursor();
    assert!(cursor.seek(SeekFrom::End(i64::MAX)).is_err());
    Ok(())
}

#[test]
fn supported_boundaries_remain_correct() -> Result<()> {
    let lua = Lua::new();
    let original = vec![Some(1), None, Some(3)];
    let actual: Vec<Option<i32>> = lua.from_value(lua.to_value(&original)?)?;
    assert_eq!(actual, original);
    let strings = HashSet::from([String::from("a"), String::from("b")]);
    let actual: HashSet<String> = lua.convert(strings.clone())?;
    assert_eq!(actual, strings);
    let table = lua.create_sequence_from([1, 2])?;
    assert_eq!(table, [1, 2]);
    assert_ne!(table, [1]);
    table.set(i64::MIN + 1, "near-minimum")?;
    table.set(i64::MAX, "maximum")?;
    assert_eq!(table.get_path::<String>("[-9223372036854775807]")?, "near-minimum");
    assert_eq!(table.get_path::<String>("[9223372036854775807]")?, "maximum");
    let weak: mlua::Table = lua.load("setmetatable({}, {__mode = 'v'})").eval()?;
    {
        let temporary = lua.create_table()?;
        weak.set(1, &temporary)?;
    }
    lua.gc_collect()?;
    lua.gc_collect()?;
    assert_eq!(weak.get::<Value>(1)?, Value::Nil);
    Ok(())
}

#[cfg(feature = "luau")]
#[test]
fn table_to_short_tuple_rejects_extra_components() -> Result<()> {
    let lua = Lua::new();
    let value = lua.pack([1.0, 2.0, 3.0])?;
    assert!(lua.from_value::<(f32, f32)>(value).is_err());
    let value = Value::Vector(mlua::Vector::new(1.0, 2.0, 3.0));
    let actual = lua.from_value::<(f32, f32, f32)>(value)?;
    assert_eq!(actual, (1.0, 2.0, 3.0));
    Ok(())
}

#[cfg(not(feature = "luau"))]
#[test]
fn removing_hook_releases_captured_resources() -> Result<()> {
    use std::sync::Arc;
    let lua = Lua::new();
    let marker = Arc::new(());
    let captured = Arc::clone(&marker);
    lua.set_hook(mlua::HookTriggers::EVERY_LINE, move |_, _| {
        let _ = Arc::strong_count(&captured);
        Ok(mlua::VmState::Continue)
    })?;
    lua.remove_hook();
    lua.gc_collect()?;
    lua.gc_collect()?;
    assert_eq!(Arc::strong_count(&marker), 1);
    Ok(())
}
