"""Post-return scorer: copy candidate, inject held-back tests, and retain results."""
from pathlib import Path
import json
import os
import re
import shlex
import shutil
import subprocess
import sys

root = Path('/work')
source = Path(sys.argv[1] if len(sys.argv) > 1 else '/work/crate').resolve()
if not source.is_relative_to(root) or not (source / 'pulldown-cmark/Cargo.toml').is_file():
    raise SystemExit('Candidate must be a repository inside /work.')
review = Path('/work/tmp/D-review')
if review.exists():
    raise SystemExit('Use a fresh /work/tmp/candidate-review location; do not overwrite an earlier run.')
shutil.copytree(source, review, ignore=shutil.ignore_patterns('.git', 'target'))
for name in ['oracle', 'neighbors']:
    dest = review / f'pulldown-cmark/tests/case_wikilink_{name}.rs'
    if dest.exists():
        raise SystemExit(f'Reserved hidden-test filename already exists: {dest}')
    shutil.copyfile(root / f'prepared/hidden/case_wikilink_{name}.rs', dest)
logs = Path('/work/results/D/frozen-runner-logs')
logs.mkdir(exist_ok=False)
env = os.environ.copy()
env.update(CARGO_HOME='/work/.cargo-home', CARGO_NET_OFFLINE='true',
           RUSTUP_AUTO_INSTALL='0', CARGO_TARGET_DIR='/work/tmp/D-target')
checks = []
for profile in ['debug', 'release']:
    flags = [] if profile == 'debug' else ['--release']
    for name, count in [('oracle', 1), ('neighbors', 4)]:
        checks.append((f'{name}-{profile}', ['cargo', 'test', '--offline', '--locked',
                       '-p', 'pulldown-cmark', '--features', 'html', *flags,
                       '--test', f'case_wikilink_{name}', '--', '--nocapture'], count))
    checks.append((f'full-suite-{profile}', ['cargo', 'test', '--offline', '--locked',
                   '-p', 'pulldown-cmark', *flags], None))
checks.append(('no-default-features', ['cargo', 'check', '--offline', '--locked',
               '-p', 'pulldown-cmark', '--no-default-features'], None))
results = []
for name, command, count in checks:
    result = subprocess.run(command, cwd=review, env=env, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True)
    output = result.stdout
    count_ok = count is None or bool(re.search(
        rf'test result: ok\. {count} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', output))
    # Retain commands, failures, warnings, and summaries; omit only routine successful test lines.
    concise = '\n'.join(line for line in output.splitlines()
                        if not re.match(r'^test .+ \.\.\. ok$', line))
    (logs / f'{name}.log').write_text(
        f'cwd={review}\n$ {shlex.join(command)}\n{concise}\nexit={result.returncode}\n'
        f'expected_hidden_count={count}; count_ok={count_ok}\n')
    results.append(dict(name=name, command=command, exit=result.returncode,
                        expected_hidden_count=count, count_ok=count_ok,
                        passed=result.returncode == 0 and count_ok))
    print(f'{name}: exit={result.returncode}, count_ok={count_ok}', flush=True)
(logs / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
raise SystemExit(0 if all(check['passed'] for check in results) else 1)
