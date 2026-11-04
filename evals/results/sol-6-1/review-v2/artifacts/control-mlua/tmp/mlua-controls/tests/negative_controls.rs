use std::collections::BTreeMap;

use mlua::{Error, FromLua, Function, Lua, LuaSerdeExt, Result, UserData, Value};

#[test]
fn decoy_1_raw_insert_append_boundary() -> Result<()> {
    let lua = Lua::new();
    let table = lua.create_table()?;
    table.raw_insert(1, "first")?;
    table.raw_insert(table.raw_len() as i64 + 1, "second")?;
    let values = table.sequence_values::<String>().collect::<Result<Vec<_>>>()?;
    assert_eq!(values, vec!["first", "second"]);
    assert!(table.raw_insert(0, "invalid").is_err());
    assert!(table.raw_insert(4, "invalid").is_err());
    assert_eq!(table.raw_len(), 2);
    println!("decoy-1: insert at len+1 succeeds for empty and nonempty sequences; indices 0 and len+2 return Err");
    Ok(())
}

#[test]
fn decoy_2_set_environment_isolates_sibling_closures() -> Result<()> {
    let lua = Lua::new();
    lua.globals().set("marker", 10)?;
    let (first, sibling): (Function, Function) = lua.load(
        "return function() return marker end, function() return marker end"
    ).eval()?;
    assert_eq!(first.environment().unwrap().to_pointer(), sibling.environment().unwrap().to_pointer());
    let env = lua.create_table_from([("marker", 20)])?;
    assert!(first.set_environment(env)?);
    assert_eq!(first.call::<i64>(())?, 20);
    assert_eq!(sibling.call::<i64>(())?, 10);
    assert_eq!(lua.globals().get::<i64>("marker")?, 10);
    assert!(sibling.set_environment(lua.create_table_from([("marker", 30)])?)?);
    assert_eq!(first.call::<i64>(())?, 20);
    assert_eq!(sibling.call::<i64>(())?, 30);
    println!("decoy-2: sibling closures initially share environment; reassignment yields independent results 20 and 30; global remains 10");
    Ok(())
}

#[test]
fn false_1_replace_nil_registry_key_allocates_independent_slot() -> Result<()> {
    let lua = Lua::new();
    let mut first = lua.create_registry_value(Value::Nil)?;
    let second = lua.create_registry_value(Value::Nil)?;
    let ordinary = lua.create_registry_value("unchanged")?;
    lua.replace_registry_value(&mut first, "alpha")?;
    assert_eq!(lua.registry_value::<String>(&first)?, "alpha");
    assert!(matches!(lua.registry_value::<Value>(&second)?, Value::Nil));
    lua.replace_registry_value(&mut first, Value::Nil)?;
    assert!(matches!(lua.registry_value::<Value>(&first)?, Value::Nil));
    lua.replace_registry_value(&mut first, "beta")?;
    assert_eq!(lua.registry_value::<String>(&first)?, "beta");
    assert!(matches!(lua.registry_value::<Value>(&second)?, Value::Nil));
    assert_eq!(lua.registry_value::<String>(&ordinary)?, "unchanged");
    println!("false-1: nil->alpha->nil->beta registry replacement works; another nil key remains nil and another live key is unchanged");
    Ok(())
}

#[derive(Debug, PartialEq)]
struct Payload(i64);
impl UserData for Payload {}

#[test]
fn false_2_failed_take_preserves_borrowed_userdata() -> Result<()> {
    let lua = Lua::new();
    let data = lua.create_userdata(Payload(42))?;
    let borrowed = data.borrow::<Payload>()?;
    assert!(matches!(data.take::<Payload>(), Err(Error::UserDataBorrowMutError)));
    assert_eq!(borrowed.0, 42);
    assert_eq!(data.borrow::<Payload>()?.0, 42);
    drop(borrowed);
    data.borrow_mut::<Payload>()?.0 = 43;
    assert_eq!(data.take::<Payload>()?, Payload(43));
    assert!(data.borrow::<Payload>().is_err());
    println!("false-2: take during shared borrow returns UserDataBorrowMutError; userdata remains readable and mutable and later take returns 43");
    Ok(())
}

#[test]
fn false_3_u8_stack_conversion_rejects_integer_overflow() -> Result<()> {
    let lua = Lua::new();
    for (chunk, integer) in [("return 256", 256), ("return -1", -1)] {
        assert!(matches!(lua.load(chunk).eval::<u8>(), Err(Error::FromLuaConversionError { .. })));
        assert!(matches!(u8::from_lua(Value::Integer(integer), &lua), Err(Error::FromLuaConversionError { .. })));
    }
    assert_eq!(lua.load("return 0").eval::<u8>()?, 0);
    assert_eq!(lua.load("return 255").eval::<u8>()?, 255);
    println!("false-3: stack and Value conversion reject 256 and -1 for u8; endpoints 0 and 255 succeed");
    Ok(())
}

#[test]
fn false_4_serde_accepts_shared_acyclic_child_table() -> Result<()> {
    let lua = Lua::new();
    let value = lua.load("local child = {answer = 42}; return {left = child, right = child}").eval::<Value>()?;
    let map: BTreeMap<String, BTreeMap<String, i64>> = lua.from_value(value)?;
    assert_eq!(map.len(), 2);
    assert_eq!(map["left"]["answer"], 42);
    assert_eq!(map["right"]["answer"], 42);
    let value = lua.load("local child = {42}; return {child, child}").eval::<Value>()?;
    let sequence: Vec<Vec<i64>> = lua.from_value(value)?;
    assert_eq!(sequence, vec![vec![42], vec![42]]);
    println!("false-4: default Serde deserialization accepts repeated child-table identity in map siblings and sequence siblings");
    Ok(())
}

#[test]
fn false_5_bind_preserves_trailing_nil_arguments() -> Result<()> {
    let lua = Lua::new();
    let function: Function = lua.load("return function(...) return select('#', ...), ... end").eval()?;
    let bound = function.bind((11, Value::Nil, 33, Value::Nil))?;
    let result: (usize, i64, Option<i64>, i64, Option<i64>, i64) = bound.call(44)?;
    assert_eq!(result, (5, 11, None, 33, None, 44));
    let result: (usize, i64, Option<i64>, i64, Option<i64>) = bound.call(())?;
    assert_eq!(result, (4, 11, None, 33, None));
    println!("false-5: bind(11,nil,33,nil) retains 4 arguments, and call(44) observes 5 arguments in the original order");
    Ok(())
}
