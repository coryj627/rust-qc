from pathlib import Path
import subprocess

ROOT = Path('/work/tmp/review')
JUST = '/target/debug/just'


def probe(name, source, args, expected_out=None, expected_code=0):
    path = ROOT / (name + '.just')
    path.write_text(source)
    result = subprocess.run(
        [JUST, '--color', 'never', '-f', str(path), *args],
        capture_output=True, text=True, timeout=10,
    )
    print(f'{name}: exit={result.returncode}, stdout={result.stdout!r}')
    if expected_out is not None:
        print(f'  expected stdout={expected_out!r}, exit={expected_code}')
    else:
        print(f'  expected a normal format diagnostic, exit={expected_code}')
    if result.stderr:
        print(f'  stderr={result.stderr[:600]!r}')


(ROOT / 'shared.just').write_text(
    "x := 'default'\n@show:\n  printf '%s:%s\\n' '{{module_path()}}' '{{x}}'\n"
)
modules = "mod a 'shared.just'\nmod b 'shared.just'\n"
probe('modules-shared', modules, ['a::show', 'b::show'], 'a:default\nb:default\n')
probe('modules-override', modules,
      ['--set', 'a::x', 'A', '--set', 'b::x', 'B', 'a::show', 'b::show'],
      'a:A\nb:B\n')
probe('function-override',
      "set unstable\nx := 'original'\nf() := x\na := f()\n",
      ['--evaluate', 'x=override'], 'a := "override"\nx := "override"\n')
probe('function-param-shadow',
      "set unstable\ny := 'global'\nx := y\nf(y) := x\n@show:\n  echo {{f('parameter')}}\n",
      [], 'global\n')
probe('function-cache',
      "set unstable\nx := uuid()\nf() := x\n@show:\n  echo {{if x == f() { 'same' } else { 'different' }}}\n",
      [], 'same\n')
probe('function-export',
      "set unstable\nexport x := 'ok'\nf() := `printf '%s' \"$x\"`\n@show:\n  echo {{f()}}\n",
      [], 'ok\n')
probe('function-dependency',
      "set unstable\nf() := is_dependency()\n@default: dep\n  echo {{is_dependency()}}:{{f()}}\n@dep:\n  echo {{is_dependency()}}:{{f()}}\n",
      [], 'true:true\nfalse:false\n')
probe('datetime-invalid', "x := datetime('%Q')\n", ['--evaluate'], expected_code=1)
probe('datetime-utc-invalid', "x := datetime_utc('%Q')\n", ['--evaluate'], expected_code=1)
probe('timestamp-invalid', "default:\n  true\n",
      ['--timestamp', '--timestamp-format', '%Q'], expected_code=1)
