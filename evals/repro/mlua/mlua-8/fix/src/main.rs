// b2bdc45: HashSet/BTreeSet of integers do not round trip through Lua.
use mlua::{FromLua, IntoLua, Lua, Result};
use std::collections::{BTreeSet, HashSet};

fn main() -> Result<()> {
    let lua = Lua::new();
    let hs: HashSet<i32> = [1, 2, 3].into_iter().collect();
    let bs: BTreeSet<i32> = [1, 2, 3].into_iter().collect();

    let hv = hs.clone().into_lua(&lua)?;
    println!("encoded HashSet: {:?}", lua.load("local t = ...; local n = 0; for k, v in pairs(t) do n = n + 1; assert(v == true) end; return n, #t").call::<(i32, i32)>(&hv)?);
    let hr = HashSet::<i32>::from_lua(hv, &lua);
    println!("HashSet round trip: {hr:?}");

    let bv = bs.clone().into_lua(&lua)?;
    let br = BTreeSet::<i32>::from_lua(bv, &lua);
    println!("BTreeSet round trip: {br:?}");

    let ok = hr.as_ref().ok() == Some(&hs) && br.as_ref().ok() == Some(&bs);
    println!("both round trips correct: {ok}");
    std::process::exit(if ok { 0 } else { 42 });
}
