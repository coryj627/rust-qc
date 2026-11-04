#![cfg(feature = "luau")]
use mlua::{Lua, Result};
use std::io::{Seek, SeekFrom};

#[test]
fn cursor_seek_end_large_offset_returns_error() -> Result<()> {
    let lua = Lua::new();
    let mut cursor = lua.create_buffer(b"x")?.cursor();
    assert!(cursor.seek(SeekFrom::End(i64::MAX)).is_err());
    Ok(())
}

#[test]
fn cursor_seek_current_large_offset_returns_error() -> Result<()> {
    let lua = Lua::new();
    let mut cursor = lua.create_buffer(b"x")?.cursor();
    cursor.seek(SeekFrom::Start(1))?;
    assert!(cursor.seek(SeekFrom::Current(i64::MAX)).is_err());
    Ok(())
}

#[test]
fn failed_vector_array_conversion_drops_initialized_elements() -> Result<()> {
    use mlua::{Error, FromLua, Value, Vector};
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    struct Positive;
    impl Drop for Positive {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::SeqCst);
        }
    }
    impl FromLua for Positive {
        fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
            let n = f32::from_lua(value, lua)?;
            if n > 0.0 {
                Ok(Self)
            } else {
                Err(Error::runtime("expected positive value"))
            }
        }
    }
    let lua = Lua::new();
    DROPS.store(0, Ordering::SeqCst);
    let table = lua.create_sequence_from([1.0, -1.0, 3.0])?;
    assert!(lua.unpack::<[Positive; 3]>(mlua::Value::Table(table)).is_err());
    assert_eq!(DROPS.load(Ordering::SeqCst), 1, "table conversion cleans up on error");
    DROPS.store(0, Ordering::SeqCst);
    let actual = lua.unpack::<[Positive; 3]>(Value::Vector(Vector::new(1.0, -1.0, 3.0)));
    assert!(actual.is_err());
    assert_eq!(
        DROPS.load(Ordering::SeqCst),
        1,
        "first initialized element must be dropped when the second conversion fails"
    );
    Ok(())
}
