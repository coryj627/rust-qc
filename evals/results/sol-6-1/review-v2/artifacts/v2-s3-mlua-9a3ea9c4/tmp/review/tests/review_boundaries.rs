use mlua::{Lua, Result};

#[test]
#[cfg(feature = "lua54")]
fn close_error_preserves_boolean_message() -> Result<()> {
    let lua = Lua::new();
    let code = "local resource <close> = setmetatable({}, {__close = function() error(true) end}); coroutine.yield()";
    let thread = lua.create_thread(lua.load(code).into_function()?)?;
    thread.resume::<()>(())?;
    let error = thread.reset(lua.load("return").into_function()?).unwrap_err();
    assert_eq!(error.to_string(), "runtime error: true");
    Ok(())
}

#[test]
#[cfg(feature = "luau")]
fn invalid_large_seek_returns_error() -> Result<()> {
    use std::io::{Seek, SeekFrom};
    let lua = Lua::new();
    let mut cursor = lua.create_buffer(b"abc")?.cursor();
    assert!(cursor.seek(SeekFrom::End(i64::MAX)).is_err());
    Ok(())
}
