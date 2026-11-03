use mlua::prelude::*;
use mlua::{Value, Table, AnyUserData, UserData};
use serde::Serialize;

#[derive(Serialize)]
enum E { T(Option<i32>, i32) }

struct T1(u64);
struct U1([u64; 64]);

fn main() -> LuaResult<()> {
    let which = std::env::args().nth(1).unwrap();
    let lua = Lua::new();
    match which.as_str() {
        "nil_key_toplevel" => {
            let t = lua.create_table()?;
            let r = t.set(Value::Nil, 1);
            println!("set(nil) -> {:?}", r.map_err(|e| e.to_string()));
        }
        "nan_key_toplevel" => {
            let t = lua.create_table()?;
            let r = t.raw_set(f64::NAN, 1);
            println!("raw_set(NaN) -> {:?}", r.map_err(|e| e.to_string()));
        }
        "table_from_nil" => {
            let r = lua.create_table_from([(Value::Nil, 1)]);
            println!("create_table_from -> {:?}", r.map_err(|e| e.to_string()));
        }
        "nil_key_limit" => {
            lua.set_memory_limit(100_000_000)?;
            let t = lua.create_table()?;
            let r = t.set(Value::Nil, 1);
            println!("with memory limit set(nil) -> {:?}", r.map_err(|e| e.to_string()));
        }
        "nil_key_cb" => {
            struct D; impl Drop for D { fn drop(&mut self) { println!("guard dropped"); } }
            let f = lua.create_function(|_, (t, k): (Table, Value)| {
                let _d = D;
                t.set(k, 1)?;
                Ok(())
            })?;
            lua.globals().set("rs_set", f)?;
            let r = lua.load("print(pcall(rs_set, {}, nil))  return 1").eval::<i32>();
            println!("{:?}", r);
        }
        "to_value_nan_key" => {
            let mut m = std::collections::BTreeMap::new();
            m.insert(Option::<i32>::None, 1);
            let opts = mlua::SerializeOptions::new().serialize_none_to_null(false);
            let r = lua.to_value_with(&m, opts);
            println!("{:?}", r.map(|_| ()).map_err(|e| e.to_string()));
        }
        "warn_err" => {
            lua.set_warning_function(|_, _, _| Err(LuaError::runtime("warning error")));
            lua.warning("hello", false);
            println!("survived warning");
        }
        "gc_clone_close" => {
            let f = lua.create_function(|lua, ()| { let l = lua.clone(); drop(l); println!("finalizer ran"); Ok(()) })?;
            lua.globals().set("fin", f)?;
            lua.load("keep = setmetatable({}, {__gc = fin})").exec()?;
            drop(lua);
            println!("dropped ok");
            return Ok(());
        }
        "gc_appdata_close" => {
            lua.set_app_data(5i32);
            let f = lua.create_function(|lua, ()| { let v = lua.app_data_ref::<i32>().map(|v| *v); println!("finalizer ran {:?}", v); Ok(()) })?;
            lua.globals().set("fin", f)?;
            lua.load("keep = setmetatable({}, {__gc = fin})").exec()?;
            drop(lua);
            println!("dropped ok");
            return Ok(());
        }
        "deep_ser" => {
            let t: Value = lua.load("local t = {} for i=1,500000 do t = {t} end return t").eval()?;
            let r = serde_json::to_string(&t).map(|s| s.len()).map_err(|e| e.to_string());
            println!("{:?}", r);
        }
        "deep_de" => {
            let t: Value = lua.load("local t = {} for i=1,500000 do t = {t} end return t").eval()?;
            #[derive(Debug, serde::Deserialize)]
            struct N(#[allow(dead_code)] Vec<N>);
            let r: LuaResult<N> = lua.from_value(t);
            println!("{}", r.is_ok());
        }
        "deep_fmt" => {
            let t: Value = lua.load("local t = {} for i=1,500000 do t = {t} end return t").eval()?;
            let s = format!("{:#?}", t);
            println!("{}", s.len());
        }
        "null_skip" => {
            let t: Value = lua.load("return {1, null, 3}").set_environment({ let e = lua.create_table()?; e.set("null", lua.null())?; e }).eval()?;
            println!("deny=true : {}", serde_json::to_string(&t.to_serializable()).unwrap());
            println!("deny=false: {}", serde_json::to_string(&t.to_serializable().deny_unsupported_types(false)).unwrap());
            let t: Value = lua.load("return {a=null, b=1}").set_environment({ let e = lua.create_table()?; e.set("null", lua.null())?; e }).eval()?;
            println!("map deny=false: {}", serde_json::to_string(&t.to_serializable().deny_unsupported_types(false).sort_keys(true)).unwrap());
        }
        "tuple_variant_none" => {
            let opts = mlua::SerializeOptions::new().serialize_none_to_null(false);
            let v = lua.to_value_with(&E::T(None, 7), opts)?;
            let t = v.as_table().unwrap().get::<Table>("T")?;
            println!("t[1]={:?} t[2]={:?}", t.raw_get::<Value>(1)?, t.raw_get::<Value>(2)?);
        }
        "mt_cache" => {
            let mut hit = 0;
            for i in 0..50 {
                lua.register_userdata_type::<T1>(|_| {})?;
                let tud = lua.create_any_userdata(T1(0x4141414141414141))?;
                { let _b = tud.borrow::<T1>()?; }
                drop(tud);
                lua.register_userdata_type::<T1>(|_| {})?; // unref first mt
                lua.gc_collect()?; lua.gc_collect()?;
                let k: usize = std::env::args().nth(2).unwrap().parse().unwrap();
                let _hold: Vec<Table> = (0..k).map(|_| lua.create_table().unwrap()).collect();
                if i > 0 { lua.register_userdata_type::<U1>(|_| {})?; }
                let uud: AnyUserData = lua.create_any_userdata(U1([7; 64]))?;
                if let Ok(b) = uud.borrow::<T1>() {
                    println!("attempt {i}: U1 userdata borrowed as T1! value {:#x}", b.0);
                    hit += 1;
                    break;
                }
            }
            println!("hit={hit}");
        }
        "unused" => { let _ = (T1(0).0, U1([0;64]).0); let _: Option<Box<dyn Fn() -> Box<dyn UserDataX>>> = None; }
        _ => panic!("unknown"),
    }
    Ok(())
}
trait UserDataX {}
struct Nope; impl UserData for Nope {}
