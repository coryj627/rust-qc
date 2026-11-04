use std::{fs, panic::{AssertUnwindSafe, catch_unwind}, path::PathBuf};
fn main() {
  let dir = PathBuf::from("/work/tmp/verify/public-api");
  fs::create_dir_all(&dir).unwrap();
  let path = dir.join("justfile");
  for (name, src, extra) in [
    ("datetime", "x := datetime('%Q')\n", vec!["--evaluate", "x"]),
    ("datetime_utc", "x := datetime_utc('%')\n", vec!["--evaluate", "x"]),
    ("timestamp", "foo:\n  echo ok\n", vec!["--timestamp", "--timestamp-format", "%Q", "foo"]),
    ("normal_function_error", "x := error('expected')\n", vec!["--evaluate", "x"]),
  ] {
    fs::write(&path, src).unwrap();
    let mut args = vec!["just", "--justfile", path.to_str().unwrap()];
    args.extend(extra);
    let result = catch_unwind(AssertUnwindSafe(|| just::run(args.iter())));
    println!("{name}: {result:?}");
  }
}
