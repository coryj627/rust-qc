use mlua::{Lua, LuaSerdeExt, Value, Table, Result, Error, UserData, UserDataMethods, MetaMethod, Function};
use mlua::{DeserializeOptions, SerializeOptions};
use serde::Serialize;
use std::collections::{HashSet, BTreeSet, BTreeMap};

#[test]
fn clear_releases_table() -> Result<()> {
    let lua = Lua::new();
    let weak: Table = lua.load("return setmetatable({}, {__mode='v'})").eval()?;
    let control = lua.create_table()?;
    weak.raw_set("control", control.clone())?;
    drop(control);
    let cleared = lua.create_table()?;
    cleared.raw_set("x", 42)?;
    weak.raw_set("cleared", cleared.clone())?;
    cleared.clear()?;
    assert_eq!(cleared.raw_get::<Value>("x")?, Value::Nil);
    drop(cleared);
    lua.gc_collect()?; lua.gc_collect()?;
    assert_eq!(weak.raw_get::<Value>("control")?, Value::Nil);
    assert_eq!(weak.raw_get::<Value>("cleared")?, Value::Nil);
    Ok(())
}

#[test]
fn relaxed_serde_preserves_nulls_deserialize() -> Result<()> {
    let lua = Lua::new();
    let input = serde_json::json!({"present":null,"items":[null,1]});
    lua.globals().set("null",Value::NULL)?;
    let value: Value = lua.load("return {present=null,items={null,1}}").eval()?;
    assert_eq!(lua.from_value::<serde_json::Value>(value.clone())?, input);
    let result: serde_json::Value = lua.from_value_with(value, DeserializeOptions::new().deny_unsupported_types(false))?;
    assert_eq!(result, input);
    Ok(())
}

#[test]
fn relaxed_serde_preserves_nulls_serialize() -> Result<()> {
    let lua = Lua::new();
    let input = serde_json::json!({"present":null,"items":[null,1]});
    lua.globals().set("null",Value::NULL)?;
    let value: Value = lua.load("return {present=null,items={null,1}}").eval()?;
    assert_eq!(serde_json::to_value(&value).unwrap(), input);
    let result = serde_json::to_value(value.to_serializable().deny_unsupported_types(false)).unwrap();
    assert_eq!(result, input);
    Ok(())
}

#[test]
fn equality_checks_complete_slice() -> Result<()> {
    let lua = Lua::new();
    let t = lua.create_sequence_from([1,2])?;
    assert!(t == [1,2]); assert!(t != [1]);
    assert!(t != [1,2,3]);
    Ok(())
}

#[test]
fn equality_checks_empty_slice() -> Result<()> {
    let lua = Lua::new();
    let t = lua.create_table()?;
    assert!(t == [] as [i32;0]);
    assert!(t != [1]);
    Ok(())
}

#[test]
fn hashset_numeric_roundtrip() -> Result<()> {
    let lua = Lua::new();
    let strings = HashSet::from(["a".to_owned(),"b".to_owned()]);
    assert_eq!(lua.convert::<HashSet<String>>(strings.clone())?, strings);
    let input = HashSet::from([1i32,2,3]);
    assert_eq!(lua.convert::<HashSet<i32>>(input.clone())?, input);
    Ok(())
}

#[test]
fn btreeset_numeric_roundtrip() -> Result<()> {
    let lua = Lua::new();
    let input = BTreeSet::from([1i32,2,3]);
    assert_eq!(lua.convert::<BTreeSet<i32>>(input.clone())?, input);
    Ok(())
}

#[derive(Serialize)]
enum Payload { Items(Option<i32>, i32), Empty() }
#[test]
fn tuple_variant_preserves_nil_positions() -> Result<()> {
    let lua = Lua::new();
    let options = SerializeOptions::new().serialize_none_to_null(false);
    let ordinary = lua.to_value_with(&(None::<i32>,7i32), options)?;
    let ordinary = ordinary.as_table().unwrap();
    assert_eq!(ordinary.raw_get::<Value>(1)?, Value::Nil);
    assert_eq!(ordinary.raw_get::<i32>(2)?, 7);
    let variant = lua.to_value_with(&Payload::Items(None,7), options)?;
    let payload: Table = variant.as_table().unwrap().raw_get("Items")?;
    assert_eq!(payload.raw_get::<Value>(1)?, Value::Nil);
    assert_eq!(payload.raw_get::<i32>(2)?, 7);
    Ok(())
}

#[test]
fn empty_tuple_variant_is_array() -> Result<()> {
    let lua = Lua::new();
    let value = lua.to_value(&Payload::Empty())?;
    assert_eq!(serde_json::to_value(value).unwrap(), serde_json::json!({"Empty":[]}));
    Ok(())
}

#[cfg(feature="send")]
#[test]
fn shared_sync_borrows_coexist() -> Result<()> {
    let lua = Lua::new();
    let ud = lua.create_any_userdata(String::from("text"))?;
    let first = ud.borrow::<String>()?;
    let second = ud.borrow::<String>()?;
    assert_eq!(*first,*second);
    Ok(())
}

#[test]
fn minimum_integer_path() -> Result<()> {
    use mlua::ObjectLike;
    let lua = Lua::new();
    let t = lua.create_table()?;
    t.raw_set(i64::MIN,42)?;
    t.raw_set(i64::MIN+1,43)?;
    t.raw_set(i64::MAX,44)?;
    assert_eq!(t.get_path::<i32>("[-9223372036854775807]")?,43);
    assert_eq!(t.get_path::<i32>("[9223372036854775807]")?,44);
    assert_eq!(t.get_path::<i32>("[-9223372036854775808]")?,42);
    Ok(())
}

#[test]
fn failed_take_preserves_userdata() -> Result<()> {
    let lua = Lua::new();
    let ud = lua.create_any_userdata(42i32)?;
    let borrowed = ud.borrow::<i32>()?;
    assert!(matches!(ud.take::<i32>(), Err(Error::UserDataBorrowMutError)));
    assert_eq!(*borrowed,42);
    drop(borrowed);
    assert_eq!(*ud.borrow::<i32>()?,42);
    assert_eq!(ud.take::<i32>()?,42);
    Ok(())
}

#[test]
fn bound_nil_keeps_argument_positions() -> Result<()> {
    let lua = Lua::new();
    let f: Function = lua.load("return function(...) return select('#', ...), ... end").eval()?;
    let bound = f.bind((11,Value::Nil,33,Value::Nil))?;
    let result = bound.call::<mlua::MultiValue>(44)?;
    assert_eq!(result.len(),6);
    assert_eq!(result.into_vec(), vec![Value::Integer(5), Value::Integer(11), Value::Nil, Value::Integer(33), Value::Nil, Value::Integer(44)]);
    Ok(())
}

#[test]
fn shared_acyclic_children_deserialize() -> Result<()> {
    let lua = Lua::new();
    let value: Value = lua.load("local child={answer=42}; return {left=child,right=child}").eval()?;
    let result: BTreeMap<String,BTreeMap<String,i32>> = lua.from_value(value)?;
    assert_eq!(result["left"]["answer"],42);
    assert_eq!(result["right"]["answer"],42);
    Ok(())
}

#[test]
fn narrow_integer_rejects_out_of_range() -> Result<()> {
    let lua = Lua::new();
    assert!(matches!(lua.load("return 256").eval::<u8>(),Err(Error::FromLuaConversionError{..})));
    assert!(matches!(lua.load("return -1").eval::<u8>(),Err(Error::FromLuaConversionError{..})));
    assert_eq!(lua.load("return 255").eval::<u8>()?,255);
    Ok(())
}

#[test]
fn nil_registry_key_can_be_replaced() -> Result<()> {
    let lua = Lua::new();
    let mut key = lua.create_registry_value(Value::Nil)?;
    assert_eq!(lua.registry_value::<Value>(&key)?,Value::Nil);
    lua.replace_registry_value(&mut key,"alpha")?;
    assert_eq!(lua.registry_value::<String>(&key)?,"alpha");
    Ok(())
}

struct EqFirst;
struct EqSecond;
impl UserData for EqFirst {
    fn add_methods<M:UserDataMethods<Self>>(methods:&mut M) {
        methods.add_meta_function(MetaMethod::Eq, |_,(_a,_b):(Value,Value)| Ok(true));
    }
}
impl UserData for EqSecond {}
#[test]
fn userdata_equality_honors_first_metamethod() -> Result<()> {
    let lua=Lua::new();
    let a=lua.create_userdata(EqFirst)?;
    let b=lua.create_userdata(EqSecond)?;
    lua.globals().set("a",a.clone())?;
    lua.globals().set("b",b.clone())?;
    #[cfg(feature="lua54")]
    assert!(lua.load("return a==b").eval::<bool>()?);
    assert!(Value::UserData(a).equals(&Value::UserData(b))?);
    Ok(())
}

struct Empty;
impl UserData for Empty {}
#[test]
fn restricted_metatable_values_do_not_convert() -> Result<()> {
    let lua=Lua::new();
    let ud=lua.create_userdata(Empty)?;
    let mt=ud.metatable()?;
    let visible=mt.pairs::<Value>().collect::<Result<Vec<_>>>()?;
    assert!(visible.iter().all(|(_,v)|matches!(v,Value::String(_))),"{visible:?}");
    let typed=mt.pairs::<String>().collect::<Result<Vec<_>>>()?;
    assert_eq!(typed.len(),visible.len());
    Ok(())
}

#[cfg(feature="lua54")]
#[test]
fn numeric_equality_preserves_integer_precision() -> Result<()> {
    let lua=Lua::new();
    let i=Value::Integer(9007199254740993);
    let n=Value::Number(9007199254740992.0);
    lua.globals().set("i",i.clone())?;
    lua.globals().set("n",n.clone())?;
    assert!(!lua.load("return i==n").eval::<bool>()?);
    assert!(!i.equals(&n)?);
    assert_ne!(i,n);
    Ok(())
}

#[test]
fn tuple_ignores_trailing_unsupported_values() -> Result<()> {
    let lua=Lua::new();
    let value:Value=lua.load("return {1,2,function() end}").eval()?;
    let options=DeserializeOptions::new().deny_unsupported_types(false);
    assert_eq!(lua.from_value_with::<Vec<i32>>(value.clone(),options)?,vec![1,2]);
    assert_eq!(lua.from_value_with::<(i32,i32)>(value,options)?,(1,2));
    Ok(())
}

#[cfg(not(feature="luau"))]
#[test]
fn remove_main_hook_releases_capture() -> Result<()> {
    use std::sync::Arc;
    let lua=Lua::new();
    let marker=Arc::new(()); let captured=marker.clone();
    lua.set_hook(mlua::HookTriggers::EVERY_LINE,move |_,_|{let _=&captured;Ok(mlua::VmState::Continue)})?;
    assert_eq!(Arc::strong_count(&marker),2);
    lua.remove_hook(); lua.gc_collect()?;lua.gc_collect()?;
    assert_eq!(Arc::strong_count(&marker),1);
    Ok(())
}

#[cfg(not(feature="luau"))]
#[test]
fn remove_thread_hook_releases_capture() -> Result<()> {
    use std::sync::Arc;
    let lua=Lua::new();
    let thread=lua.create_thread(lua.load("return 1").into_function()?)?;
    let marker=Arc::new(());let captured=marker.clone();
    thread.set_hook(mlua::HookTriggers::EVERY_LINE,move |_,_|{let _=&captured;Ok(mlua::VmState::Continue)})?;
    thread.remove_hook(); lua.gc_collect()?;lua.gc_collect()?;
    assert_eq!(Arc::strong_count(&marker),1);
    drop(thread);
    Ok(())
}

#[cfg(feature="lua54")]
#[test]
fn close_error_keeps_boolean_value() -> Result<()> {
    let lua=Lua::new();
    let function=lua.load("local x <close> = setmetatable({}, {__close=function() error(true) end}); coroutine.yield()").into_function()?;
    let thread=lua.create_thread(function)?;
    thread.resume::<()>(())?;
    let error=thread.reset(lua.load("return 1").into_function()?).unwrap_err();
    assert_eq!(error.to_string(),"runtime error: true");
    Ok(())
}

#[cfg(feature="luau")]
#[test]
fn vector_array_failure_drops_initialized_elements() -> Result<()> {
    use std::sync::atomic::{AtomicUsize,Ordering};
    static DROPS: AtomicUsize=AtomicUsize::new(0);
    struct Positive;
    impl Drop for Positive {fn drop(&mut self){DROPS.fetch_add(1,Ordering::SeqCst);}}
    impl mlua::FromLua for Positive {
        fn from_lua(value:Value,lua:&Lua)->Result<Self>{
            let n=f32::from_lua(value,lua)?;
            if n>0.0 {Ok(Self)}else{Err(Error::runtime("nonpositive"))}
        }
    }
    let lua=Lua::new();
    DROPS.store(0,Ordering::SeqCst);
    assert!(lua.unpack::<[Positive;3]>(Value::Table(lua.create_sequence_from([1.0,-1.0,3.0])?)).is_err());
    assert_eq!(DROPS.load(Ordering::SeqCst),1);
    DROPS.store(0,Ordering::SeqCst);
    assert!(lua.unpack::<[Positive;3]>(Value::Vector(mlua::Vector::new(1.0,-1.0,3.0))).is_err());
    assert_eq!(DROPS.load(Ordering::SeqCst),1);
    Ok(())
}

#[cfg(feature="luau")]
#[test]
fn vector_tuple_rejects_extra_components() -> Result<()> {
    let lua=Lua::new();
    let value=Value::Vector(mlua::Vector::new(1.0,2.0,3.0));
    assert_eq!(lua.from_value::<(f32,f32,f32)>(value.clone())?,(1.0,2.0,3.0));
    let table=Value::Table(lua.create_sequence_from([1.0,2.0,3.0])?);
    assert!(lua.from_value::<(f32,f32)>(table).is_err());
    assert!(lua.from_value::<(f32,f32)>(value).is_err());
    Ok(())
}

#[cfg(feature="luau")]
#[test]
fn seek_end_overflow_returns_error() -> Result<()> {
    use std::io::{Seek,SeekFrom};
    let lua=Lua::new();
    let mut cursor=lua.create_buffer(b"abc")?.cursor();
    assert!(cursor.seek(SeekFrom::End(1)).is_err());
    assert!(cursor.seek(SeekFrom::End(i64::MAX)).is_err());
    Ok(())
}

#[cfg(feature="luau")]
#[test]
fn seek_current_overflow_returns_error() -> Result<()> {
    use std::io::{Seek,SeekFrom};
    let lua=Lua::new();
    let mut cursor=lua.create_buffer(b"abc")?.cursor();
    cursor.seek(SeekFrom::Start(1))?;
    assert!(cursor.seek(SeekFrom::Current(i64::MAX)).is_err());
    Ok(())
}
