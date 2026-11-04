import pathlib, subprocess
root = pathlib.Path('/work/tmp/review')
cases = [
 ('datetime', "x := datetime('%Q')\n", ['--evaluate', 'x']),
 ('datetime-utc', "x := datetime_utc('%')\n", ['--evaluate', 'x']),
 ('override', "set unstable\nx := 'original'\nf() := x\n@main:\n  echo {{x}} {{f()}}\n", ['x=override']),
 ('memoized', "set unstable\nx := `printf x >> counter; cat counter`\nf() := x\n@main:\n  echo {{x}} {{f()}} {{f()}}\n", []),
 ('function-export', "set unstable\nexport X := 'value'\nf() := `printf '%s' \"${X-unset}\"`\n@main:\n  echo {{f()}}\n", []),
 ('function-is-dep', "set unstable\nf() := is_dependency()\nmain: dep\n@dep:\n  echo {{is_dependency()}} {{f()}}\n", []),
 ('function-lazy', "set unstable\nset lazy\nx := 'foo'\nf() := x\n@main:\n  echo {{f()}}\n", []),
]
for name, text, args in cases:
 path = root/name
 path.mkdir(exist_ok=True)
 (path/'justfile').write_text(text)
 if name == 'memoized' and (path/'counter').exists(): (path/'counter').unlink()
 result = subprocess.run(['/target/debug/just', '--color', 'never', *args], cwd=path, text=True, capture_output=True, timeout=5)
 print(name, 'exit=', result.returncode, 'stdout=', repr(result.stdout), 'stderr=', repr(result.stderr))
