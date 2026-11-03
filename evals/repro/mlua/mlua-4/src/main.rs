// 08e2663: scoped userdata methods and scoped callbacks that outlive the scope.
use mlua::{Function, Lua, ObjectLike, Result, UserDataMethods};
use std::cell::RefCell;
use std::rc::Rc;

struct OnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        self.0.take().unwrap()();
    }
}

// Defect 1: the method matches on the userdata pointer and reads storage take_userdata moved out.
fn escaped_method() -> Result<()> {
    let lua = Lua::new();
    let (ud, get) = lua.scope(|scope| {
        let ud = scope.create_any_userdata(String::from("scoped userdata payload"), |reg| {
            reg.add_method("get", |_, s: &String, ()| Ok(s.clone()));
        })?;
        let get = ud.get::<Function>("get")?;
        assert_eq!(get.call::<String>(&ud)?, "scoped userdata payload");
        Ok((ud, get))
    })?;
    println!("scope ended, calling escaped method");
    let r = get.call::<mlua::Value>(&ud);
    println!("escaped method returned: {r:?}");
    Ok(())
}

// Defect 2: a callback registered during scope teardown is never invalidated.
fn teardown_callback() -> Result<()> {
    let lua = Lua::new();
    let data = String::from("borrowed by a scoped callback");
    let escaped: Rc<RefCell<Option<Function>>> = Rc::new(RefCell::new(None));
    lua.scope(|scope| {
        let data_ref = &data;
        let esc = escaped.clone();
        let guard = OnDrop(Some(move || {
            let f = scope.create_function(move |_, ()| Ok(data_ref.clone())).unwrap();
            *esc.borrow_mut() = Some(f);
        }));
        scope.create_function(move |_, ()| {
            let _ = &guard;
            Ok(())
        })?;
        Ok(())
    })?;
    drop(data);
    let f = escaped.borrow_mut().take().expect("teardown callback was registered");
    println!("scope ended and borrowed String dropped, calling teardown callback");
    let r = f.call::<mlua::Value>(());
    println!("teardown callback returned: {r:?}");
    Ok(())
}

fn main() -> Result<()> {
    match std::env::args().nth(1).unwrap().as_str() {
        "method" => escaped_method(),
        "teardown" => teardown_callback(),
        _ => panic!("mode"),
    }
}
