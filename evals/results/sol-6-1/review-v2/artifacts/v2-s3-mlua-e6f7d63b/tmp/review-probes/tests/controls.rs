use mlua::{Lua, LuaSerdeExt, Result, Table, Value};
use std::collections::{BTreeSet, HashSet};

#[test]
fn tables_without_clear_are_collected() -> Result<()> {
    let lua = Lua::new();
    let weak: Table = lua.load("setmetatable({}, {__mode = 'v'})").eval()?;
    for i in 1..=10 {
        let table = lua.create_table()?;
        weak.set(i, &table)?;
    }
    lua.gc_collect()?;
    assert_eq!(weak.pairs::<Value, Value>().count(), 0);
    Ok(())
}

#[test]
fn matching_sequences_compare_equal() -> Result<()> {
    let lua = Lua::new();
    for input in [vec![], vec![1], vec![1, 2]] {
        let table = lua.create_sequence_from(input.clone())?;
        assert_eq!(table, input.as_slice());
    }
    Ok(())
}

#[test]
fn string_and_non_array_integer_sets_round_trip() -> Result<()> {
    let lua = Lua::new();
    for input in [
        BTreeSet::from([0]),
        BTreeSet::from([2, 10]),
        BTreeSet::from([-2, -1]),
    ] {
        assert_eq!(lua.convert::<BTreeSet<i32>>(input.clone())?, input);
    }
    let input = HashSet::from(["one".to_owned(), "two".to_owned()]);
    assert_eq!(lua.convert::<HashSet<String>>(input.clone())?, input);
    Ok(())
}

#[test]
fn default_serde_preserves_nulls() -> Result<()> {
    let lua = Lua::new();
    let input = serde_json::json!({"present": null, "array": [1, null, 3]});
    let value = lua.to_value_with(
        &input,
        mlua::SerializeOptions::new().detect_serde_json_arbitrary_precision(true),
    )?;
    assert_eq!(lua.from_value::<serde_json::Value>(value.clone())?, input);
    assert_eq!(
        serde_json::to_value(value).map_err(mlua::Error::external)?,
        input
    );
    Ok(())
}
