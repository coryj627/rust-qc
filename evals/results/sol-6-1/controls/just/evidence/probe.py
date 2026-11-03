#!/usr/bin/python3
"""Safe public CLI counterexamples for the seven controls; no crate edits."""
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys

ROOT = Path('/work/prepared/evidence')
JUST = '/target/debug/just'
IDS = ['decoy-1', 'decoy-2', 'false-1', 'false-2', 'false-3', 'false-4', 'false-5']
control = sys.argv[1]
assert control in IDS
case_dir = ROOT / control
case_dir.mkdir(exist_ok=True)
records = []
environment = {k: v for k, v in os.environ.items() if not k.startswith('JUST_')}
environment['LC_ALL'] = 'C'
environment['TMPDIR'] = '/work/tmp'

def fixture(name, text):
    path = case_dir / name
    path.write_text(text)
    return path

def run(name, path, args, expected_stdout, expected_code=0, stderr_contains=None, env=None):
    command = [JUST, '--color', 'never', '--justfile', str(path), *args]
    child_env = environment.copy()
    if env:
        child_env.update(env)
    result = subprocess.run(command, cwd=case_dir, env=child_env, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=10)
    record = {'case': name, 'command': command, 'environment_overrides': env or {},
              'exit_code': result.returncode, 'stdout': result.stdout.decode(),
              'stderr': result.stderr.decode(), 'expected_stdout': expected_stdout.decode(),
              'expected_exit_code': expected_code}
    records.append(record)
    (case_dir / 'results.json').write_text(json.dumps(records, indent=2) + '\n')
    assert result.returncode == expected_code, record
    assert result.stdout == expected_stdout, record
    if stderr_contains:
        assert stderr_contains in result.stderr, record
    assert b'panicked at' not in result.stderr, record
    assert b'internal error' not in result.stderr.lower(), record
    print(f"PASS {name}: exit={result.returncode}, stdout={result.stdout!r}")
    print('  ' + shlex.join(command))
    if env:
        print('  environment: ' + repr(env))
    if result.stderr:
        print('  stderr=' + repr(result.stderr))

if control == 'decoy-1':
    p = fixture('justfile', """all: (emit 'alpha') (emit 'beta') (emit 'alpha')

emit value:
    @printf '%s\\n' '{{value}}'
""")
    run('distinct dependency values survive deduplication', p, ['all'], b'alpha\nbeta\n')
    run('distinct direct invocation values survive deduplication', p,
        ['emit', 'alpha', 'emit', 'beta', 'emit', 'alpha'], b'alpha\nbeta\n')
elif control == 'decoy-2':
    p = fixture('justfile', """value := "can't stop"

check:
    @printf '<%s>\\n' {{quote(value)}}
""")
    run('embedded apostrophe and space form one shell argument', p, ['check'], b"<can't stop>\n")
    run('empty input remains one empty shell argument', p, ['value=', 'check'], b'<>\n')
    run('multiple embedded apostrophes remain literal', p,
        ["value='a' 'b'", 'check'], b"<'a' 'b'>\n")
elif control == 'false-1':
    p = fixture('justfile', """selected_then := if 'x' == 'x' { 'ok-then' } else { error('unselected-else') }
selected_else := if 'x' == 'y' { error('unselected-then') } else { 'ok-else' }
""")
    run('then selection skips error in else branch', p, ['--evaluate', 'selected_then'], b'ok-then')
    run('else selection skips error in then branch', p, ['--evaluate', 'selected_else'], b'ok-else')
    p2 = fixture('selected-error.just', "x := if 'x' == 'x' { error('selected-error') } else { 'unused' }\n")
    run('selected error is active, so the sentinel is effective', p2, ['--evaluate', 'x'],
        b'', expected_code=1, stderr_contains=b'selected-error')
elif control == 'false-2':
    fixture('.env', 'CONTROL_INHERITED=file-default\n')
    p = fixture('justfile', """set dotenv-load
value := env('CONTROL_INHERITED')

check:
    @printf 'env=<%s> shell=<%s>\\n' '{{value}}' "$CONTROL_INHERITED"
""")
    run('explicitly empty inherited value wins', p, ['check'], b'env=<> shell=<>\n',
        env={'CONTROL_INHERITED': ''})
    run('nonempty inherited value wins', p, ['check'], b'env=<host> shell=<host>\n',
        env={'CONTROL_INHERITED': 'host'})
    p2 = fixture('override.just', p.read_text().replace('set dotenv-load', 'set dotenv-load\nset dotenv-override'))
    run('explicit override setting activates the same dotenv fixture', p2, ['check'],
        b'env=<file-default> shell=<file-default>\n', env={'CONTROL_INHERITED': ''})
elif control == 'false-3':
    p = fixture('justfile', """set positional-arguments

show +values:
    @printf 'zero=<%s> count=<%s>\\n' "$0" "$#"
    @printf 'arg=<%s>\\n' "$@"
""")
    run('recipe name fills zero and both argument boundaries survive', p,
        ['show', 'hello world', 'tail'],
        b'zero=<show> count=<2>\narg=<hello world>\narg=<tail>\n')
elif control == 'false-4':
    p = fixture('justfile', "x := replace_regex('abc', '[', 'x')\n")
    run('malformed regex is a normal diagnosed function error', p, ['--evaluate', 'x'], b'',
        expected_code=1, stderr_contains=b'Call to function `replace_regex` failed:')
    p2 = fixture('valid.just', "x := replace_regex('abca', 'a', 'x')\n")
    run('valid regex follows the replacement path', p2, ['--evaluate', 'x'], b'xbcx')
elif control == 'false-5':
    p = fixture('justfile', """two_lf := `printf 'payload\\n\\n'`
spaces := `printf 'payload  \\n'`
two_crlf := `printf 'payload\\r\\n\\r\\n'`
no_newline := `printf 'payload  '`
""")
    run('only one of two terminal LF bytes is removed', p, ['--evaluate', 'two_lf'], b'payload\n')
    run('spaces before final newline survive', p, ['--evaluate', 'spaces'], b'payload  ')
    run('only one of two terminal CRLF pairs is removed', p, ['--evaluate', 'two_crlf'], b'payload\r\n')
    run('trailing spaces without a newline survive', p, ['--evaluate', 'no_newline'], b'payload  ')

(case_dir / 'results.json').write_text(json.dumps(records, indent=2) + '\n')
print(f'{control}: {len(records)} cases passed')
