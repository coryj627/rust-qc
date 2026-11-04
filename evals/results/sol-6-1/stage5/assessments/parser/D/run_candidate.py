from pathlib import Path
import os, sys, json, shutil, hashlib, subprocess, shlex, time, re
letter = sys.argv[1]
assert letter in 'ABCD' and len(letter) == 1
base = Path('/work/results') / letter
crate = Path('/work/candidates') / letter
env = os.environ.copy()
env.update(RUSTUP_AUTO_INSTALL='0', CARGO_NET_OFFLINE='true', CARGO_TARGET_DIR=f'/work/tmp/assessment-{letter}-target')

def snapshot():
    return {str(p.relative_to(crate)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(crate.rglob('*')) if p.is_file()}

def counts(log):
    results = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out', log)
    if not results:
        return None
    names = ['passed', 'failed', 'ignored', 'measured', 'filtered_out']
    return dict(zip(names, [sum(int(r[i]) for r in results) for i in range(5)]))

before = snapshot()
(base / 'source-before').mkdir(exist_ok=True)
for p in before:
    dest = base / 'source-before' / p
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(crate / p, dest)
(base / 'submission-before.json').write_text(json.dumps(before, indent=2) + '\n')
for path in ['run_checks.sh', 'run_api_tests.sh', 'public_api.rs']:
    shutil.copy2(Path('/work/prepared/hidden') / path, base / 'frozen' / path)
shutil.copy2('/work/gate/check.sh', base / 'gate/check.sh')
record = {'submission': str(crate), 'report': f'/work/reports/{letter}.md', 'checks': [], 'adaptations': ['Replaced /work/crate with this candidate path and selected a fresh per-candidate frozen log directory.', 'No injected tests or modified submission source. Frozen public API runner creates its own external consumer.', 'Gate uses the original small_config crate, --all-features baseline; heapless/DECOM-specific instructions do not apply.'], 'limits': [], 'adapter': {'source': str(base / 'adapter/adapter.rs'), 'tests': str(base / 'adapter/tests.rs'), 'review': str(base / 'adapter/review.md'), 'available': letter in 'BD', 'frozen_adapter_suite': False, 'note': 'scoring.md defines no category adapter tests; seven preserved tests are supplemental and excluded from hidden counts.'}}

if '--resume' in sys.argv:
    record = json.loads((base / 'record.json').read_text())

def save():
    (base / 'record.json').write_text(json.dumps(record, indent=2) + '\n')

def run(name, argv, logpath, timeout=240, extra_env=None):
    for existing in record['checks']:
        if existing['name'] == name:
            return existing
    runenv = env.copy()
    if extra_env: runenv.update(extra_env)
    explicit_env = {k: runenv[k] for k in ['RUSTUP_AUTO_INSTALL', 'CARGO_NET_OFFLINE', 'CARGO_TARGET_DIR']}
    if extra_env: explicit_env.update(extra_env)
    command = shlex.join(['env'] + [f'{k}={v}' for k,v in explicit_env.items()] + argv)
    result = {'name': name, 'command': command, 'argv': argv, 'cwd': '/work', 'exit_code': None, 'log': str(logpath), 'environment': explicit_env, 'timeout_seconds': timeout, 'adaptation_or_limit': None}
    started = time.monotonic()
    with open(logpath, 'wb') as log:
        try:
            process = subprocess.run(argv, env=runenv, cwd='/work', stdout=log, stderr=subprocess.STDOUT, timeout=timeout)
            result['exit_code'] = process.returncode
            result['observed_outcome'] = 'completed_successfully' if process.returncode == 0 else 'nonzero_exit_requires_log_review'
        except subprocess.TimeoutExpired:
            result['observed_outcome'] = 'environmental_timeout_incomplete'
            result['adaptation_or_limit'] = 'Timeout killed the wrapper; complete log up to timeout retained. No numeric process exit observed.'
    result['duration_seconds'] = round(time.monotonic() - started, 3)
    text = Path(logpath).read_text(errors='replace')
    c = counts(text)
    if c: result['test_counts'] = c
    record['checks'].append(result)
    with open(base / 'commands.jsonl', 'a') as f: f.write(json.dumps(result) + '\n')
    save()
    print(f'{letter}: {name}: exit={result["exit_code"]}', flush=True)
    return result

frozen_dir = base / 'frozen'
run('frozen_orchestrator', ['/work/prepared/hidden/run_checks.sh', str(crate), str(frozen_dir)], frozen_dir / 'runner.log', 240, {'CARGO_TARGET_DIR': '/work/tmp/small-config-submission-target'})
commands = {
    'library': ['cargo','check','--manifest-path',str(crate / 'Cargo.toml'),'--offline','--lib'],
    'submitted-tests': ['cargo','test','--manifest-path',str(crate / 'Cargo.toml'),'--offline','--all-targets'],
    'doctests': ['cargo','test','--manifest-path',str(crate / 'Cargo.toml'),'--offline','--doc'],
    'public-api': ['/work/prepared/hidden/run_api_tests.sh', str(crate)],
    'fmt': ['cargo','fmt','--manifest-path',str(crate / 'Cargo.toml'),'--check'],
    'clippy': ['cargo','clippy','--manifest-path',str(crate / 'Cargo.toml'),'--offline','--all-targets'],
}
for line in (frozen_dir / 'status.tsv').read_text().splitlines():
    label, status = line.split('\t')
    if any(c['name'] == label for c in record['checks']):
        continue
    checkenv = {'RUSTUP_AUTO_INSTALL':'0','CARGO_NET_OFFLINE':'true','CARGO_TARGET_DIR': '/work/tmp/small-config-api-target' if label == 'public-api' else '/work/tmp/small-config-submission-target'}
    log = frozen_dir / f'{label}.log'
    item = {'name':label,'command':shlex.join(['env']+[f'{k}={v}' for k,v in checkenv.items()]+commands[label]), 'argv':commands[label], 'cwd':'/work', 'environment':checkenv, 'exit_code':int(status), 'log':str(log), 'observed_outcome':'completed_successfully' if int(status) == 0 else 'nonzero_exit_requires_log_review', 'adaptation_or_limit': 'External consumer created by frozen runner; runner target overrides orchestrator target.' if label == 'public-api' else None}
    c = counts(log.read_text(errors='replace'))
    if c: item['test_counts'] = c
    record['checks'].append(item)
    with open(base / 'commands.jsonl','a') as f: f.write(json.dumps(item)+'\n')
    if label == 'public-api': record['hidden_test_counts'] = c
save()
run('reported_strict_clippy', ['cargo','clippy','--manifest-path',str(crate / 'Cargo.toml'),'--offline','--all-targets','--','-D','warnings'], base / 'own/strict-clippy.log')
run('reported_strict_rustdoc', ['cargo','doc','--manifest-path',str(crate / 'Cargo.toml'),'--offline','--no-deps'], base / 'own/strict-rustdoc.log', extra_env={'RUSTDOCFLAGS':'-D warnings'})
run('archived_gate', ['bash','/work/gate/check.sh',str(crate)], base / 'gate/gate.log')
if letter in 'BD':
    run('supplemental_category_adapter', ['cargo','test','--manifest-path',str(base / 'adapter/Cargo.toml'),'--offline','--test','categories','--','--test-threads=1'], base / 'adapter/tests.log')
else:
    log = base / 'adapter/unrun.log'
    log.write_text('UNRUN: no public machine-readable error category API. See review.md. Frozen public API checks still ran.\n')
    record['checks'].append({'name':'supplemental_category_adapter','command':shlex.join(['cargo','test','--manifest-path',str(base / 'adapter/Cargo.toml'),'--offline','--test','categories','--','--test-threads=1']), 'exit_code':None,'log':str(log), 'observed_outcome':'unrun_api_cannot_be_classified', 'adaptation_or_limit':'Only line() is publicly available. Private textual reason is not classified; all seven supplemental category tests unrun. This optional API is not required by the case.'})
after = snapshot()
(base / 'submission-after.json').write_text(json.dumps(after, indent=2) + '\n')
changes = {'modified':[p for p in before if p in after and before[p] != after[p]], 'added':[p for p in after if p not in before], 'deleted':[p for p in before if p not in after], 'note':'Normal Cargo verification can create/update Cargo.lock; assessment-created harness lockfiles and tests live under results or /work/tmp and are not candidate authorship.'}
record['submission_changes'] = changes
(base / 'submission-changes.json').write_text(json.dumps(changes, indent=2) + '\n')
save()
