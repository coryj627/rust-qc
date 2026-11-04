"""Run held-back and relevant compatibility checks; retain exact commands/output."""
from pathlib import Path
import json
import os
import subprocess
import sys

if len(sys.argv) != 3:
    raise SystemExit('usage: python3 run_checks.py STAGED_SOURCE LOG_LABEL')
source = Path(sys.argv[1]).resolve()
label = sys.argv[2]
if not label.replace('-', '').replace('_', '').isalnum():
    raise SystemExit('log label must contain letters, digits, hyphens, or underscores')
if not source.is_relative_to(Path('/work/tmp')):
    raise SystemExit('run only on staged copies under /work/tmp')
logs = Path('/work/prepared/logs')
logs.mkdir(exist_ok=True)
env = os.environ.copy()
env.update(RUSTUP_AUTO_INSTALL='0', CARGO_HOME='/work/.cargo-home',
           CARGO_TARGET_DIR=f'/work/tmp/target-{label}', TMPDIR='/work/tmp/cargo-temp',
           XDG_RUNTIME_DIR='/work/tmp/runtime', CARGO_TERM_COLOR='never', NO_COLOR='1', USER='just-case')
for directory in (env['TMPDIR'], env['XDG_RUNTIME_DIR']):
    Path(directory).mkdir(parents=True, exist_ok=True)
base = ['cargo', 'test', '--offline', '--locked', '--test', 'integration']
steps = [
    ('compile', base + ['--no-run']),
    ('upstream', base + ['hidden_just_6::', '--', '--test-threads=1', '--nocapture']),
    ('neighbors', base + ['hidden_just_6_neighbors::', '--', '--test-threads=1', '--nocapture']),
    ('functions', base + ['functions::', '--', '--test-threads=1']),
    ('backticks', base + ['backticks::', '--', '--test-threads=1']),
    ('existing-dry-run', base + ['misc::dry_run', '--', '--exact']),
]
results = []
for step, command in steps:
    log = logs/f'{label}-{step}.log'
    with log.open('w') as output:
        output.write('cwd: ' + str(source) + '\n')
        output.write('command: ' + ' '.join(command) + '\n')
        output.write('environment: ' + json.dumps({k:env[k] for k in ['RUSTUP_AUTO_INSTALL','CARGO_HOME','CARGO_TARGET_DIR','TMPDIR','XDG_RUNTIME_DIR','CARGO_TERM_COLOR','NO_COLOR','USER']}, sort_keys=True) + '\n\n')
        output.flush()
        try:
            result = subprocess.run(command, cwd=source, env=env, stdout=output,
                                    stderr=subprocess.STDOUT, timeout=600)
            code = result.returncode
        except subprocess.TimeoutExpired:
            output.write('\nCHECK TIMED OUT after 600 seconds\n')
            code = 124
        output.write(f'\nexit_code: {code}\n')
    results.append({'step':step, 'command':command, 'exit_code':code, 'log':str(log)})
    print(f'{label} {step}: exit {code}; {log}', flush=True)
    if step == 'compile' and code:
        print(f'{label}: runtime checks unavailable because compilation failed', flush=True)
        break
(logs/f'{label}-summary.json').write_text(json.dumps({'source':str(source), 'environment':{k:env[k] for k in ['RUSTUP_AUTO_INSTALL','CARGO_HOME','CARGO_TARGET_DIR','TMPDIR','XDG_RUNTIME_DIR','USER']}, 'results':results}, indent=2)+'\n')
raise SystemExit(int(any(r['exit_code'] for r in results)))
