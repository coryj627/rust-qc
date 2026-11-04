use mlua::{Error, FromLua, Function, Lua, LuaSerdeExt, Result, Table, Value};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn decoy_1_foreign_registry_replace_is_rejected() -> Result<()> {
    let a = Lua::new();
    let b = Lua::new();
    let mut foreign = a.create_registry_value("state-a")?;
    let local = b.create_registry_value("state-b")?;
    assert_eq!(foreign.id(), local.id(), "exercise colliding numeric IDs");
    let id = foreign.id();
    assert!(a.owns_registry_value(&foreign));
    assert!(!b.owns_registry_value(&foreign));
    let err = b.replace_registry_value(&mut foreign, "overwritten").unwrap_err();
    assert!(matches!(err, Error::MismatchedRegistryKey));
    assert_eq!(foreign.id(), id);
    assert_eq!(a.registry_value::<String>(&foreign)?, "state-a");
    assert_eq!(b.registry_value::<String>(&local)?, "state-b");
    println!("decoy-1: colliding registry IDs {id}; replacement rejected with {err}; both originals preserved");
    Ok(())
}

#[test]
fn decoy_2_shared_acyclic_tables_deserialize() -> Result<()> {
    let lua = Lua::new();
    let root: Table = lua.load("local child = {value = 7}; return {left = child, right = child}").eval()?;
    let left: Table = root.get("left")?;
    let right: Table = root.get("right")?;
    assert_eq!(left.to_pointer(), right.to_pointer());
    let actual: serde_json::Value = lua.from_value(Value::Table(root))?;
    assert_eq!(actual, serde_json::json!({"left":{"value":7},"right":{"value":7}}));
    let seq: Value = lua.load("local child = {7, 8}; return {child, child}").eval()?;
    let actual_seq: Vec<Vec<i64>> = lua.from_value(seq)?;
    assert_eq!(actual_seq, vec![vec![7, 8], vec![7, 8]]);
    let cycle: Value = lua.load("local t = {}; t.self = t; return t").eval()?;
    let err = lua.from_value::<serde_json::Value>(cycle).unwrap_err();
    assert!(err.to_string().contains("recursive table detected"));
    println!("decoy-2: default deserialization preserved both shared map branches and both shared sequence elements; actual self-cycle rejected: {err}");
    Ok(())
}

#[test]
fn false_1_eval_runtime_error_does_not_retry() -> Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        calls = 0
        function bump_then_fail()
            calls = calls + 1
            error("probe failure")
        end
    "#).exec()?;
    let err = lua.load("bump_then_fail()").eval::<Value>().unwrap_err();
    assert!(matches!(&err, Error::RuntimeError(s) if s.contains("probe failure")));
    assert_eq!(lua.globals().get::<i64>("calls")?, 1);
    lua.load("calls = calls + 1").eval::<()>()?;
    assert_eq!(lua.globals().get::<i64>("calls")?, 2);
    println!("false-1: failing expression executed once and returned RuntimeError; statement fallback executed once");
    Ok(())
}

#[test]
fn false_2_chained_bind_keeps_argument_order_and_nil() -> Result<()> {
    let lua = Lua::new();
    let f: Function = lua.load("function(...) return select('#', ...), ... end").eval()?;
    let first = f.bind(("first", Option::<i64>::None))?;
    let second = first.bind("second")?;
    let actual: (usize, String, Option<i64>, String, String) = second.call("call")?;
    assert_eq!(actual, (4, "first".into(), None, "second".into(), "call".into()));
    let original: (usize, String, Option<i64>, String) = first.call("original")?;
    assert_eq!(original, (3, "first".into(), None, "original".into()));
    println!("false-2: chained bind yielded (first, nil, second, call), argc=4; original bound function preserved");
    Ok(())
}

#[test]
fn false_3_environment_change_isolates_sibling_closures() -> Result<()> {
    let lua = Lua::new();
    lua.globals().set("marker", "old")?;
    let (f, sibling, replace_shared_env): (Function, Function, Function) = lua.load(r#"
        local _ENV = _ENV
        local function f() return marker end
        local function sibling() return marker end
        local function replace_shared_env(env) _ENV = env end
        return f, sibling, replace_shared_env
    "#).call(())?;
    let shared = lua.create_table()?;
    shared.set("marker", "shared")?;
    replace_shared_env.call::<()>(shared)?;
    assert_eq!(f.call::<String>(())?, "shared");
    assert_eq!(sibling.call::<String>(())?, "shared");
    let env = lua.create_table()?;
    env.set("marker", "new")?;
    assert!(f.set_environment(env.clone())?);
    assert_eq!(f.call::<String>(())?, "new");
    assert_eq!(sibling.call::<String>(())?, "shared");
    assert_eq!(lua.globals().get::<String>("marker")?, "old");
    assert_eq!(f.environment().unwrap().to_pointer(), env.to_pointer());
    env.set("marker", "updated")?;
    assert_eq!(f.call::<String>(())?, "updated");
    assert_eq!(sibling.call::<String>(())?, "shared");
    let later_shared = lua.create_table()?;
    later_shared.set("marker", "later")?;
    replace_shared_env.call::<()>(later_shared)?;
    assert_eq!(sibling.call::<String>(())?, "later");
    assert_eq!(f.call::<String>(())?, "updated");
    println!("false-3: replacing shared _ENV initially affected both siblings; set_environment detached f; sibling followed later shared-cell replacement while f stayed isolated");
    Ok(())
}

fn has_recursive_mut_callback(err: &Error) -> bool {
    match err {
        Error::RecursiveMutCallback => true,
        Error::CallbackError { cause, .. } => has_recursive_mut_callback(cause),
        _ => false,
    }
}

#[test]
fn false_4_recursive_mut_callback_returns_error_without_panic() -> Result<()> {
    let lua = Lua::new();
    let calls = Rc::new(Cell::new(0));
    let observed_calls = calls.clone();
    let f = lua.create_function_mut(move |lua, recurse: bool| {
        observed_calls.set(observed_calls.get() + 1);
        if recurse {
            lua.globals().get::<Function>("reenter")?.call::<()>(false)?;
        }
        Ok(())
    })?;
    lua.globals().set("reenter", f.clone())?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.call::<()>(true)));
    let err = result.expect("recursive call must not panic").unwrap_err();
    assert!(has_recursive_mut_callback(&err), "wrong error: {err:?}");
    assert_eq!(calls.get(), 1, "reentry must be rejected before executing FnMut");
    f.call::<()>(false)?;
    assert_eq!(calls.get(), 2, "borrow must be released after error");
    println!("false-4: reentry returned RecursiveMutCallback in CallbackError chain without Rust panic; callback remained callable");
    Ok(())
}

fn is_range_error(err: Error) -> bool {
    matches!(err, Error::FromLuaConversionError { message: Some(s), .. } if s == "out of range")
}

#[test]
fn false_5_narrow_integer_conversions_reject_overflow() -> Result<()> {
    let lua = Lua::new();
    for n in [-1, 256] {
        assert!(is_range_error(u8::from_lua(Value::Integer(n), &lua).unwrap_err()));
        assert!(is_range_error(lua.load(n.to_string()).eval::<u8>().unwrap_err()));
    }
    assert!(is_range_error(i8::from_lua(Value::Integer(128), &lua).unwrap_err()));
    assert!(is_range_error(lua.load("128").eval::<i8>().unwrap_err()));
    assert_eq!(lua.load("0").eval::<u8>()?, 0);
    assert_eq!(lua.load("255").eval::<u8>()?, 255);
    assert_eq!(lua.load("-128").eval::<i8>()?, -128);
    assert_eq!(lua.load("127").eval::<i8>()?, 127);
    println!("false-5: u8 rejected -1 and 256; i8 rejected 128 on both Value and stack paths; exact endpoints accepted");
    Ok(())
}
