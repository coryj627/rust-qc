// 752955c: Table == slice ignores trailing slice elements.
use mlua::{Lua, Result};

fn main() -> Result<()> {
    let lua = Lua::new();
    let t = lua.load("{1, 2, 3}").eval::<mlua::Table>()?;
    let empty = lua.create_table()?;
    let cases = [
        ("{1,2,3} == [1,2,3]", t == [1, 2, 3], true),
        ("{1,2,3} == [1,2]", t == [1, 2], false),
        ("{1,2,3} == [1,2,3,4,5,6]", t == [1, 2, 3, 4, 5, 6], false),
        ("{} == [1]", empty == [1], false),
        ("{} == []", empty == [0i32; 0], true),
    ];
    let mut wrong = 0;
    for (name, got, want) in cases {
        let flag = if got == want { "ok" } else { wrong += 1; "WRONG" };
        println!("{name}: got {got}, correct {want}: {flag}");
    }
    println!("wrong results: {wrong}");
    std::process::exit(if wrong > 0 { 42 } else { 0 });
}
