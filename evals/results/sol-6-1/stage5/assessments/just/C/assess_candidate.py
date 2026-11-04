from pathlib import Path
import subprocess, os, json, hashlib, shutil, sys, re, time, platform
letter, phase = sys.argv[1:]
assert letter in 'ABCD'
result = Path('/work/results') / letter
result.mkdir(exist_ok=True)
source = Path('/work/candidates') / letter
scratch = Path('/work/tmp') / ('candidate-' + letter + '-score')
state_path = result / 'record.json'
state = json.loads(state_path.read_text()) if state_path.exists() else {'candidate': letter, 'source': str(source), 'checks': [], 'adaptations': [], 'limitations': []}
env = os.environ.copy()
env.update(CARGO_HOME='/work/.cargo-home', CARGO_TARGET_DIR='/work/tmp/just-6-control-target', RUSTUP_AUTO_INSTALL='0', TMPDIR='/work/tmp', CARGO_NET_OFFLINE='true', CARGO_BUILD_JOBS='4')
def save():
    state_path.write_text(json.dumps(state, indent=2) + '\n')
def manifest(root):
    out = {}
    for p in sorted(root.rglob('*')):
        rel = str(p.relative_to(root))
        if '.git' in p.relative_to(root).parts or 'target' in p.relative_to(root).parts: continue
        if p.is_symlink(): out[rel] = {'symlink': os.readlink(p)}
        elif p.is_file(): out[rel] = {'sha256': hashlib.sha256(p.read_bytes()).hexdigest(), 'mode': oct(p.stat().st_mode & 0o777)}
    return out

def run(name, command, cwd=source, extra_env=None, timeout=650):
    runenv = env.copy()
    if extra_env: runenv.update(extra_env)
    path = result / (name + '.log')
    print(f'{letter}: starting {name}', flush=True)
    started = time.time()
    with path.open('w') as f:
        f.write('cwd: ' + str(cwd) + '\ncommand: ' + json.dumps(command) + '\n')
        f.write('environment: ' + json.dumps({k: runenv.get(k) for k in ('CARGO_HOME','CARGO_TARGET_DIR','RUSTUP_AUTO_INSTALL','TMPDIR','CARGO_NET_OFFLINE','CARGO_BUILD_JOBS','USER','RUSTDOCFLAGS')}) + '\n')
        f.flush()
        try:
            proc = subprocess.run(command, cwd=cwd, env=runenv, stdout=f, stderr=subprocess.STDOUT, timeout=timeout)
            code, timed_out = proc.returncode, False
        except subprocess.TimeoutExpired:
            code, timed_out = 124, True
            f.write('\nASSESSOR TIMEOUT\n')
        except FileNotFoundError as e:
            code, timed_out = 127, False
            f.write(str(e) + '\n')
        f.write(f'\nexit_code: {code}\n')
    content = path.read_text(errors='replace')
    counts = [dict(zip(('passed','failed','ignored','measured','filtered_out'), map(int,m))) for m in re.findall(r'test result: .*? (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out', content)]
    record = {'check': name, 'command': command, 'cwd': str(cwd), 'exit_code': code, 'log': str(path), 'duration_seconds': round(time.time()-started,2), 'test_summaries': counts, 'observed_outcome': 'pass' if code == 0 else 'needs_log_classification', 'adaptation_or_limit': None}
    if timed_out: record['observed_outcome'] = 'environmental_block_timeout'
    state['checks'].append(record)
    save()
    print(f'{letter}: {name} exit {code}; test summaries: {counts}', flush=True)
    return code

if phase == 'init':
    shutil.copyfile('/work/results/assess_candidate.py', result / 'assess_candidate.py')
    (result / 'source-before.json').write_text(json.dumps(manifest(source),indent=2)+'\n')
    shutil.copytree(source, scratch, symlinks=True, ignore=shutil.ignore_patterns('.git','target'))
    state['scratch'] = str(scratch)
    state['adaptations'] = ['Only candidate source path and scratch/log directories substituted in frozen procedure. Hidden runner and its environment unchanged. Hidden tests injected only into scratch.', 'Archived gate runs on the original just workspace with default --all-features. Heapless/DECOM/public-API-library special cases do not apply.']
    state['limitations'] = ['Linux/aarch64 only; PowerShell test early returns on Linux do not validate Windows.', 'No minimum-Rust-version checks or installations.']
    save()
    run('tool-environment', ['bash','-c','rustc --version; cargo --version; rustup toolchain list; uname -a; command -v mdbook || true; command -v mdbook-linkcheck || true; /work/clock'])
    run('apply-hidden', ['python3','/work/prepared/hidden/apply_hidden.py',str(scratch)])
elif phase == 'hidden':
    run('frozen-hidden-runner', ['python3','/work/prepared/hidden/run_checks.py',str(scratch),str(result / 'hidden')], timeout=800)
    p = result / 'hidden/results.json'
    if p.exists():
        state['frozen_checks'] = json.loads(p.read_text())
        for r in state['frozen_checks']:
            log = Path(r['log']).read_text()
            r['observed_outcome'] = 'pass' if r['effective_exit_code'] == 0 else ('compile_failure' if r['check']=='compile' else 'needs_log_classification')
            r['adaptation_or_limit'] = 'Fresh scratch copy; unchanged frozen runner environment and command arguments.'
        state['hidden_test_counts'] = {r['check']: r['test_counts_pass_fail_ignore'] for r in state['frozen_checks'] if r['check'] in ('upstream','neighbors')}
        save()
elif phase == 'own':
    run('own-build', ['cargo','build','--offline','--locked','-p','just','--bin','just'])
    base = ['cargo','test','--offline','--locked','-p','just','--test','integration']
    code = run('own-functions', base + ['functions::','--','--nocapture'])
    if code:
        run('own-functions-user-audit',base+['functions::','--','--nocapture'],extra_env={'USER':'audit'})
        state['adaptations'].append('Re-ran functions suite with USER=audit after the unmodified environment run, matching candidate reports; both outcomes retained.')
        save()
    run('own-dry-run-exact',base+['misc::dry_run','--','--exact','--nocapture'])
    run('own-backticks',base+['backticks::','--','--nocapture'])
    run('own-shell-module',base+['shell::','--','--nocapture'])
    run('own-dry-run-filter',base+['dry_run','--','--nocapture'])
    run('own-format', ['cargo','fmt','--all','--','--check'])
    run('full-integration-on-score-copy',base,cwd=scratch)
    run('own-doctests',['cargo','test','--offline','--locked','--workspace','--doc'])
elif phase == 'gate':
    run('archived-gate',['bash','/work/gate/check.sh',str(source)],timeout=800)
elif phase == 'supplemental':
    run('own-backtick-broad', ['cargo','test','--offline','--locked','-p','just','backtick','--','--nocapture'])
    run('own-forbidden-words', ['bash','bin/forbid'])
    run('own-shellcheck', ['shellcheck','www/install.sh'])
    doc_copy = Path('/work/tmp') / ('candidate-' + letter + '-docs')
    shutil.copytree(source, doc_copy, symlinks=True, ignore=shutil.ignore_patterns('.git','target'))
    run('own-generate-book', ['cargo','run','--offline','--locked','--package','generate-book'],cwd=doc_copy)
    run('own-mdbook-en', ['mdbook','build','book/en'],cwd=doc_copy)
    run('own-mdbook-zh', ['mdbook','build','book/zh'],cwd=doc_copy)
    run('environment-probe', ['bash','-c', 'id; locale; command -v rg || true; command -v shellcheck || true; command -v mdbook || true; /work/clock'])
    state['adaptations'].append('Book generation runs on a separate fresh pristine source copy to preserve submission documentation: ' + str(doc_copy))
    state['limitations'].append('Network/install/release CI actions excluded; unavailable ripgrep, shellcheck and mdbook are recorded as tooling blocks. No tools installed.')
    save()
elif phase == 'finalize':
    before = json.loads((result/'source-before.json').read_text())
    after = manifest(source)
    (result / 'source-after.json').write_text(json.dumps(after,indent=2)+'\n')
    changes = [{'path': k,'before':before.get(k),'after':after.get(k)} for k in sorted(before.keys()|after.keys()) if before.get(k)!=after.get(k)]
    state['submission_changes_during_verification'] = changes
    state['scratch_injected_files'] = ['tests/lib.rs (two module registrations)','tests/case_just_6_upstream.rs','tests/case_just_6_neighbors.rs']
    state['adapters'] = {'applicability':'not_applicable', 'reason':'This is a command-line dry-run case, not the public-API library case.'}
    save()
else: raise ValueError(phase)
