"""Run offline checks on a copy where apply_hidden.py has registered the oracle."""
from pathlib import Path
import json
import os
import re
import subprocess
import sys

root, logs = map(lambda s: Path(s).resolve(), sys.argv[1:3])
logs.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env.update(CARGO_HOME='/work/.cargo-home', CARGO_TARGET_DIR='/work/tmp/just-6-control-target',
           RUSTUP_AUTO_INSTALL='0', TMPDIR='/work/tmp', CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='4')
base = ['cargo', 'test', '--offline', '--locked', '-p', 'just', '--test', 'integration']
checks = [
    ('compile', base + ['--no-run']),
    ('upstream', base + ['case_just_6_upstream::', '--', '--nocapture']),
    ('neighbors', base + ['case_just_6_neighbors::', '--', '--nocapture']),
    ('existing-shell', base + ['functions::shell_', '--', '--nocapture']),
    ('existing-dry-run', base + ['misc::dry_run', '--', '--exact', '--nocapture']),
    ('existing-backticks', base + ['backticks::', '--', '--nocapture']),
]
records = []
for name, cmd in checks:
    path = logs / (name + '.log')
    with path.open('w') as stream:
        stream.write('cwd: ' + str(root) + '\n')
        stream.write('env: CARGO_HOME=/work/.cargo-home CARGO_TARGET_DIR=/work/tmp/just-6-control-target '
                     'RUSTUP_AUTO_INSTALL=0 TMPDIR=/work/tmp CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=4\n')
        stream.write('command: ' + ' '.join(cmd) + '\n')
        stream.flush()
        result = subprocess.run(cmd, cwd=root, env=env, stdout=stream, stderr=subprocess.STDOUT)
        stream.write(f'\nexit_code: {result.returncode}\n')
    effective_code = result.returncode
    counts = None
    if name in ('upstream', 'neighbors'):
        match = re.search(r'test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored;', path.read_text())
        counts = list(map(int, match.groups())) if match else None
        required = 2 if name == 'upstream' else 9
        if counts is None or counts[0] + counts[1] != required or counts[2] != 0:
            effective_code = effective_code or 1
            with path.open('a') as stream:
                stream.write(f'Test-count validation failed: expected {required} executed and 0 ignored; saw {counts}.\n')
    records.append({'check': name, 'command': cmd, 'exit_code': result.returncode,
                    'effective_exit_code': effective_code, 'test_counts_pass_fail_ignore': counts,
                    'log': str(path)})
    (logs/'results.json').write_text(json.dumps(records, indent=2)+'\n')
    print(f'{name}: exit {result.returncode}; {path}', flush=True)
    if name == 'compile' and result.returncode:
        print('Compilation/dependency failure: behavior checks not executed.', flush=True)
        sys.exit(2)
sys.exit(1 if any(r['effective_exit_code'] for r in records) else 0)
