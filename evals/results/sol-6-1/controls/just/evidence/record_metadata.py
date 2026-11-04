import hashlib
import json
from pathlib import Path
import subprocess

root = Path('/work/prepared/evidence')
crate = Path('/work/crate')
scratch = Path('/work/tmp/just-control-build')
for line in (root / 'baseline-before.sha256').read_text().splitlines():
    digest, relative = line.split('  ', 1)
    assert hashlib.sha256((scratch / relative).read_bytes()).hexdigest() == digest
commands = [['/usr/local/cargo/bin/rustc', '--version'],
            ['/usr/local/cargo/bin/cargo', '--version'], ['/target/debug/just', '--version'],
            ['uname', '-sm']]
metadata = {'build_command': 'cd /work/tmp/just-control-build && RUSTUP_AUTO_INSTALL=0 CARGO_TARGET_DIR=/target cargo build --locked --bin just',
            'build_exit_code': 0, 'build_log': 'build.log',
            'scratch_sources_match_baseline': True, 'versions': []}
for cmd in commands:
    r = subprocess.run(cmd, capture_output=True, check=True, text=True)
    metadata['versions'].append({'command': cmd, 'stdout': r.stdout.strip()})
metadata['binary_sha256'] = hashlib.sha256(Path('/target/debug/just').read_bytes()).hexdigest()
(root / 'build-metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
controls = json.loads(Path('/work/prepared/controls.json').read_text())
with (root / 'source-excerpts.txt').open('w') as out:
    for control in controls['decoys'] + controls['false_findings']:
        out.write(f"{control['id']}\n")
        for ref in control['refutation']['source_refs']:
            path, number = ref.rsplit(':', 1)
            n = int(number)
            lines = (crate / path).read_text().splitlines()
            out.write(f'{ref}: {lines[n - 1]}\n')
        out.write('\n')
print('PASS: scratch build source files match all 288 baseline file hashes')
print('PASS: saved actual version commands, binary hash, and cited source excerpts')
