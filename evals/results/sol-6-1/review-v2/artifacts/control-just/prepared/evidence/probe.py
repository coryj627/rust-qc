#!/usr/bin/env python3
"""Small functional CLI probes against the unmodified scratch build of just 1.50.0."""
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
BIN = Path('/target/debug/just')
IDS = ['decoy-1', 'decoy-2', 'false-1', 'false-2', 'false-3', 'false-4', 'false-5']


def probe(control):
    records = []
    def run(args, stdout, code=0, stderr_contains=None, fixture='justfile', env_value=None):
        env = {k: v for k, v in os.environ.items() if not k.startswith('JUST_')}
        env.pop('CONTROL_EMPTY', None)
        if env_value is not None:
            env['CONTROL_EMPTY'] = env_value
        command = [str(BIN), '--color', 'never', '--no-dotenv', '--justfile',
                   str(ROOT / control / fixture), *args]
        result = subprocess.run(command, cwd=ROOT / control, env=env,
                                stdin=subprocess.DEVNULL, capture_output=True, timeout=10)
        expected_stdout = stdout.encode()
        passed = (result.returncode == code and result.stdout == expected_stdout and
                  ((stderr_contains.encode() in result.stderr) if stderr_contains is not None
                   else result.stderr == b''))
        records.append({'command': command, 'cwd': str(ROOT / control),
                        'environment': {'JUST_*': 'removed', 'CONTROL_EMPTY': env_value},
                        'exit_code': result.returncode,
                        'stdout': result.stdout.decode(), 'stderr': result.stderr.decode(),
                        'stdout_hex': result.stdout.hex(),
                        'expected': {'exit_code': code, 'stdout': stdout,
                                     'stderr_contains': stderr_contains}, 'passed': passed})
        return passed

    if control == 'decoy-1':
        run(['default'], 'one\ntwo\ndone\n')
        run(['emit', 'one', 'emit', 'two', 'emit', 'one'], 'one\ntwo\n')
    elif control == 'decoy-2':
        run(['--evaluate', 'value'], '  payload \t')
        run(['--evaluate', 'no_newline'], 'payload \t')
        run(['--evaluate', 'multiline'], 'payload\n')
    elif control == 'false-1':
        run(['show', '--name='], '<>\n')
        run(['show'], '<fallback>\n')
        run(['show', '--name='], '<>\n', fixture='required.justfile')
        run(['show'], '', code=1, stderr_contains='Recipe `show` requires option `--name`',
            fixture='required.justfile')
    elif control == 'false-2':
        run(['--evaluate', 'truthy'], 'chosen-then')
        run(['--evaluate', 'falsy'], 'chosen-else')
        run(['--evaluate', 'value'], '', code=1, stderr_contains='SELECTED_ERROR',
            fixture='selected.justfile')
    elif control == 'false-3':
        run(['--evaluate', 'value'], '[]', env_value='')
        run(['--evaluate', 'value'], '[fallback]')
        run(['--evaluate', 'value'], '[present]', env_value='present')
    elif control == 'false-4':
        run(['gather'], '', code=1,
            stderr_contains='Recipe `gather` got 0 positional arguments but takes at least 1')
        run(['gather', 'one', 'two'], '<one two>\n')
        run(['gather'], '<>\n', fixture='star.justfile')
    elif control == 'false-5':
        run(['build', 'xdevx'], '', code=1, stderr_contains='does not match pattern')
        run(['build', 'xprod'], '', code=1, stderr_contains='does not match pattern')
        run(['build', 'devx'], '', code=1, stderr_contains='does not match pattern')
        run(['build', 'dev'], 'dependency-ran\nbody:dev\n')
        run(['build', 'prod'], 'dependency-ran\nbody:prod\n')
    else:
        raise ValueError(control)
    passed = all(record['passed'] for record in records)
    log = {'control': control,
           'version': subprocess.check_output([str(BIN), '--version'], text=True).strip(),
           'passed': passed, 'cases': records}
    (ROOT / control / 'probe-log.json').write_text(json.dumps(log, indent=2) + '\n')
    print(f'{control}: {"PASS" if passed else "FAIL"} ({len(records)} cases)')
    if not passed:
        print(json.dumps(log, indent=2))
    return passed

if __name__ == '__main__':
    chosen = sys.argv[1:] or IDS
    outcomes = [probe(control) for control in chosen]
    sys.exit(0 if all(outcomes) else 1)
