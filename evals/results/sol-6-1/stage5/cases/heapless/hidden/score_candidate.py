#!/usr/bin/env python3
"""Apply held-back tests only to a fresh copy of a returned candidate."""
from pathlib import Path
import json
import shutil
import subprocess
import sys

if len(sys.argv) != 3:
    raise SystemExit('usage: score_candidate.py CANDIDATE_ROOT NEW_RUN_DIRECTORY')
candidate = Path(sys.argv[1]).resolve()
run = Path(sys.argv[2]).resolve()
if not candidate.is_relative_to('/work') or not (candidate / 'Cargo.toml').is_file():
    raise SystemExit('candidate must be a crate within /work')
if not run.is_relative_to('/work/tmp') or run.exists() or run.is_relative_to(candidate):
    raise SystemExit('run directory must be new, under /work/tmp, and outside candidate')
run.mkdir(parents=True)
source = run / 'source'
shutil.copytree(candidate, source, ignore=shutil.ignore_patterns('target', '.git'))
hidden = Path(__file__).resolve().parent
results = {}

def check(label, command):
    log = run / 'logs' / (label + '.log')
    args = [sys.executable, str(hidden / 'run_logged.py'), str(log), str(source), str(run / 'target'), *command]
    results[label] = subprocess.run(args).returncode

# Run candidate-owned tests before exposing hidden files to its copied source.
check('existing-and-candidate-tests', ['cargo', 'test', '--offline', '--quiet'])
check('no-std-default-library', ['cargo', 'check', '--offline', '--lib', '--no-default-features'])
for name in ('heldback_truncate_upstream.rs', 'heldback_truncate_spec.rs'):
    target = source / 'tests' / name
    if target.exists():
        raise SystemExit(f'refusing to overwrite candidate file: {target}')
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(hidden / name, target)
check('upstream-oracle', ['cargo', 'test', '--offline', '--test', 'heldback_truncate_upstream', '--', '--exact', 'truncate_invalid', '--nocapture'])
check('spec-neighbors', ['cargo', 'test', '--offline', '--test', 'heldback_truncate_spec', '--', '--nocapture'])
for name in ('heldback_truncate_upstream.rs', 'heldback_truncate_spec.rs'):
    if (source / 'tests' / name).read_bytes() != (hidden / name).read_bytes():
        raise SystemExit(f'held-back test changed while running: {name}')
(run / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
print(json.dumps(results, indent=2))
sys.exit(0 if all(code == 0 for code in results.values()) else 1)
