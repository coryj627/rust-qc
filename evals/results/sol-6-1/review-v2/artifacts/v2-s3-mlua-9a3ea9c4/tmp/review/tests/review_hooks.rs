#![cfg(not(feature = "luau"))]
use mlua::{HookTriggers, Lua, Result, VmState};
use std::sync::Arc;

#[test]
fn removing_hook_releases_capture() -> Result<()> {
    let lua = Lua::new();
    let data = Arc::new(());
    let captured = data.clone();
    lua.set_hook(HookTriggers::EVERY_LINE, move |_, _| {
        let _keep_alive = &captured;
        Ok(VmState::Continue)
    })?;
    lua.remove_hook();
    lua.gc_collect()?;
    lua.gc_collect()?;
    assert_eq!(Arc::strong_count(&data), 1);
    Ok(())
}

#[test]
fn removing_thread_hook_releases_capture() -> Result<()> {
    let lua = Lua::new();
    let thread = lua.create_thread(lua.load("return 1").into_function()?)?;
    let data = Arc::new(());
    let captured = data.clone();
    thread.set_hook(HookTriggers::EVERY_LINE, move |_, _| {
        let _keep_alive = &captured;
        Ok(VmState::Continue)
    })?;
    thread.remove_hook();
    lua.gc_collect()?;
    lua.gc_collect()?;
    assert_eq!(Arc::strong_count(&data), 1);
    Ok(())
}
