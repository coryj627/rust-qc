use clap::Parser;
use std::{fs, panic::{self, AssertUnwindSafe}, path::Path};
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
fn main() {
    panic::set_hook(Box::new(|_| {}));
    let path = Path::new("/work/tmp/fuzz.just");
    let alphabet = b"ab :='\"{}()\n\t`+-/?!#\\";
    let seeds = [
        "x := 'v'\nr:\n echo {{x}}\n", "f(x) := x\na := f('z')\n",
        "[arg('x', pattern='.*')]\nr x='v':\n @echo {{x}}\n",
        "x := f\"hello { 'world' }\"\n", "set shell := ['sh', '-cu']\n",
        "a: b\nb:\n", "x := if 'a' == 'b' { 'yes' } else { 'no' }\n",
    ];
    let mut state = 0x1234_5678_90ab_cdef;
    for i in 0..300000usize {
        let mut input = if i < 200000 { seeds[next(&mut state) as usize % seeds.len()].to_owned() }
            else { String::new() };
        let changes = next(&mut state) % 10 + 1;
        for _ in 0..changes {
            let at = next(&mut state) as usize % (input.len()+1);
            let c = alphabet[next(&mut state) as usize % alphabet.len()] as char;
            match next(&mut state) % 3 {
                0 if at < input.len() => { input.remove(at); },
                1 if at < input.len() => { input.replace_range(at..at+1, &c.to_string()); },
                _ => input.insert(at,c),
            }
        }
        fs::write(path,&input).unwrap();
        if panic::catch_unwind(AssertUnwindSafe(|| just::summary::summary(path))).is_err() {
            println!("PANIC input {input:?}"); return;
        }
        if panic::catch_unwind(AssertUnwindSafe(|| just::unindent(&input))).is_err() {
            println!("UNINDENT PANIC {input:?}"); return;
        }
        let args: Vec<_> = std::iter::once("just").chain(input.split_whitespace()).collect();
        if panic::catch_unwind(AssertUnwindSafe(|| just::Arguments::try_parse_from(args))).is_err() {
            println!("ARGS PANIC {input:?}"); return;
        }
        let _ = serde_json::from_str::<just::request::Request>(&input);
    }
    println!("300000 mutated inputs passed summary, unindent, Arguments and Request");
}
