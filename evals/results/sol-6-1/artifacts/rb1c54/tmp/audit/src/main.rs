use mlua::{Lua, Result, Value};
fn main() -> Result<()> {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let lua = Lua::new();
    match mode.as_str() {
        "nil" => println!("{:?}", lua.create_table()?.raw_set(Value::Nil, 1)),
        "nan" => println!("{:?}", lua.create_table()?.raw_set(f64::NAN, 1)),
        "nil_from" => println!("{:?}", lua.create_table_from([(Value::Nil, 1)])),
        "metatable" => {
            let t = lua.create_table()?;
            let s = lua.create_string("this is a string, not a table")?;
            let other = Lua::new();
            let _padding = other.create_table()?;
            let mt = other.create_table()?;
            println!("setting metatable with foreign reference; string {s:?}");
            t.set_metatable(Some(mt))?;
            println!("metatable: {:?}", t.metatable());
            t.set("foo", 3)?;
        }
        "eq" => {
            let t = lua.create_sequence_from([1])?;
            println!("[1] == [1,2]: {}", t == [1, 2]);
            println!("empty == [1]: {}", lua.create_table()? == [1]);
        }
        "clear" => {
            lua.gc_collect()?;
            let before = lua.used_memory();
            for _ in 0..10000 {
                let t = lua.create_sequence_from([42])?;
                t.clear()?;
            }
            lua.gc_collect()?;
            println!("memory before {before}, after {}", lua.used_memory());
        }
        #[cfg(feature = "send")]
        "userdata_sync" => {
            use std::cell::Cell;
            fn assert_sync<T: Sync>() {}
            assert_sync::<mlua::UserDataRef<Cell<u64>>>();
            assert_sync::<mlua::UserDataRefMut<Cell<u64>>>();
            let ud = lua.create_any_userdata(Cell::new(0u64))?;
            let borrowed = ud.borrow::<Cell<u64>>()?;
            std::thread::scope(|scope| {
                for _ in 0..4 {
                    let b = &borrowed;
                    scope.spawn(move || { for _ in 0..100000 { let v = b.get(); std::hint::spin_loop(); b.set(v + 1); } });
                }
            });
            println!("Cell counter: {} (expected 400000)", borrowed.get());
        }
        "scoped_method" => {
            use mlua::{UserData, UserDataMethods, ObjectLike, Function};
            struct Item(String);
            impl UserData for Item {
                fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
                    methods.add_method("get", |_, item, ()| Ok(item.0.clone()));
                }
            }
            let (ud, method) = lua.scope(|scope| {
                let ud = scope.create_userdata(Item("owned text from scoped userdata".repeat(10)))?;
                let method: Function = ud.get("get")?;
                Ok((ud, method))
            })?;
            println!("ordinary borrow after scope: {:?}", ud.borrow::<Item>().err());
            println!("saved method after scope: {:?}", method.call::<std::string::String>(ud));
        }
        "warning" => {
            #[cfg(feature = "lua54")]
            {
                lua.set_warning_function(|_, _, _| Err(mlua::Error::runtime("warning callback failed")));
                lua.warning("test", false);
            }
        }
        #[cfg(feature = "luau")]
        "buffer_alias" => {
            use mlua::LuaSerdeExt;
            let buffer = lua.create_buffer(b"hello")?;
            BUFFER.with(|slot| *slot.borrow_mut() = Some(buffer.clone()));
            let result = lua.from_value::<BytesProbe>(Value::Buffer(buffer));
            println!("buffer probe: {result:?}");
        }
        "raw_seti" => println!("{:?}", lua.create_table()?.raw_seti(usize::MAX, 1)),
        _ => println!("unknown mode {mode}"),
    }
    Ok(())
}

#[cfg(feature = "luau")]
thread_local! { static BUFFER: std::cell::RefCell<Option<mlua::Buffer>> = const { std::cell::RefCell::new(None) }; }
#[cfg(feature = "luau")]
#[derive(Debug)]
struct BytesProbe;
#[cfg(feature = "luau")]
impl<'de> serde::Deserialize<'de> for BytesProbe {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> std::result::Result<Self,D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = BytesProbe;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("bytes") }
            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> std::result::Result<BytesProbe,E> {
                let before = bytes[0];
                BUFFER.with(|slot| slot.borrow().as_ref().unwrap().write_bytes(0,b"X"));
                println!("immutable slice changed from {} to {}", before as char, bytes[0] as char);
                Ok(BytesProbe)
            }
        }
        de.deserialize_bytes(Visitor)
    }
}
