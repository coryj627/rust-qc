use mlua::{DeserializeOptions, Lua, LuaSerdeExt, Result, Table, Value};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[test]
fn clear_does_not_retain_tables() -> Result<()> {
    let lua = Lua::new();
    let weak: Table = lua.load("setmetatable({}, {__mode = 'v'})").eval()?;
    for i in 1..=10 {
        let table = lua.create_table()?;
        weak.set(i, &table)?;
        table.clear()?;
    }
    lua.gc_collect()?;
    let retained = weak.pairs::<Value, Value>().count();
    assert_eq!(
        retained, 0,
        "cleared tables should be collected after their Rust handles are dropped"
    );
    Ok(())
}

#[test]
fn permissive_deserialization_preserves_nulls() -> Result<()> {
    let lua = Lua::new();
    let value = lua.to_value(&serde_json::json!({"present": null, "number": 3}))?;
    let expected = lua.from_value::<serde_json::Value>(value.clone())?;
    let actual = lua.from_value_with::<serde_json::Value>(
        value,
        DeserializeOptions::new().deny_unsupported_types(false),
    )?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn integer_set_round_trip() -> Result<()> {
    let lua = Lua::new();
    for input in [
        BTreeSet::from([1]),
        BTreeSet::from([1, 2, 3]),
        BTreeSet::from([0, 1, 10]),
    ] {
        let value = lua.pack(input.clone())?;
        let actual = lua.unpack::<BTreeSet<i32>>(value);
        assert_eq!(actual?, input);
    }
    Ok(())
}

#[test]
fn integer_hash_set_round_trip() -> Result<()> {
    let lua = Lua::new();
    let input = HashSet::from([1, 2, 3]);
    let actual = lua.unpack::<HashSet<i32>>(lua.pack(input.clone())?)?;
    assert_eq!(actual, input);
    Ok(())
}

#[test]
fn permissive_deserialization_preserves_array_nulls() -> Result<()> {
    let lua = Lua::new();
    let value = lua.to_value(&vec![Some(1), None, Some(3)])?;
    let actual = lua.from_value_with::<Vec<Option<i32>>>(
        value,
        DeserializeOptions::new().deny_unsupported_types(false),
    )?;
    assert_eq!(actual, vec![Some(1), None, Some(3)]);
    Ok(())
}

#[test]
fn permissive_deserialization_preserves_null_map_values() -> Result<()> {
    let lua = Lua::new();
    let input = BTreeMap::from([("a".to_owned(), None::<i32>)]);
    let value = lua.to_value_with(
        &input,
        mlua::SerializeOptions::new().detect_serde_json_arbitrary_precision(true),
    )?;
    let actual = lua.from_value_with::<BTreeMap<String, Option<i32>>>(
        value,
        DeserializeOptions::new().deny_unsupported_types(false),
    )?;
    assert_eq!(actual, input);
    Ok(())
}

#[test]
fn table_slice_equality_checks_lengths() -> Result<()> {
    let lua = Lua::new();
    for (table_values, slice_values) in [
        (vec![], vec![1]),
        (vec![1], vec![1, 2]),
        (vec![1, 2], vec![1, 2, 3]),
    ] {
        let table = lua.create_sequence_from(table_values.clone())?;
        assert_ne!(
            table,
            slice_values.as_slice(),
            "different lengths: {table_values:?} and {slice_values:?}"
        );
    }
    Ok(())
}

#[test]
fn permissive_serialization_preserves_nulls() -> Result<()> {
    let lua = Lua::new();
    let input = serde_json::json!({"present": null, "array": [1, null, 3]});
    let value = lua.to_value_with(
        &input,
        mlua::SerializeOptions::new().detect_serde_json_arbitrary_precision(true),
    )?;
    let actual = serde_json::to_value(value.to_serializable().deny_unsupported_types(false))
        .map_err(mlua::Error::external)?;
    assert_eq!(actual, input);
    Ok(())
}

#[test]
fn get_path_accepts_minimum_integer_key() -> Result<()> {
    use mlua::ObjectLike;
    let lua = Lua::new();
    let table = lua.create_table()?;
    table.set(i64::MIN, "minimum")?;
    assert_eq!(
        table.get_path::<String>("[-9223372036854775808]")?,
        "minimum"
    );
    Ok(())
}

#[test]
fn empty_tuple_variant_serializes_as_array() -> Result<()> {
    #[derive(serde::Serialize)]
    enum Message {
        Items(),
    }
    let lua = Lua::new();
    let input = Message::Items();
    let expected = serde_json::to_value(&input).map_err(mlua::Error::external)?;
    let actual = serde_json::to_value(lua.to_value(&input)?).map_err(mlua::Error::external)?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn tuple_variant_preserves_indices_when_none_serializes_to_nil() -> Result<()> {
    #[derive(serde::Serialize)]
    enum Message {
        Items(Option<i32>, i32),
    }
    let lua = Lua::new();
    let options = mlua::SerializeOptions::new().serialize_none_to_null(false);
    let plain = lua.to_value_with(&(None::<i32>, 7), options)?;
    let tagged = lua.to_value_with(&Message::Items(None, 7), options)?;
    let plain = plain.as_table().unwrap();
    let tagged: Table = tagged.as_table().unwrap().get("Items")?;
    assert_eq!(tagged.raw_get::<Value>(1)?, plain.raw_get::<Value>(1)?);
    assert_eq!(tagged.raw_get::<Value>(2)?, plain.raw_get::<Value>(2)?);
    Ok(())
}

#[test]
fn nonempty_table_is_not_equal_to_longer_slice() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_sequence_from([1, 2])?;
    assert_ne!(table, [1, 2, 3]);
    Ok(())
}
