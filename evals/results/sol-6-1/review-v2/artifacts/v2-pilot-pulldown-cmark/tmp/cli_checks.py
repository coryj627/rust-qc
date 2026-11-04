from pathlib import Path
import subprocess
root = Path('/work/tmp')
a, b = root / 'a.md', root / 'b.md'
a.write_text('alpha\n')
b.write_text('beta\n')
binary = '/target/debug/pulldown-cmark'
r = subprocess.run([binary, str(a), str(b)], capture_output=True)
print('two files:', r.returncode, repr(r.stdout))
assert r.stdout == b'<p>alpha</p>\n<p>alpha\nbeta</p>\n'
r = subprocess.run([binary], input=b'alpha\n\xffbeta\n', capture_output=True)
print('invalid UTF-8 stdin:', r.returncode, repr(r.stdout), repr(r.stderr))
assert r.returncode == 0 and r.stdout == b''
with open('/dev/full', 'wb') as full:
    r = subprocess.run([binary, str(a)], stdout=full, stderr=subprocess.PIPE)
print('failed stdout:', r.returncode, repr(r.stderr))
assert r.returncode == 0
r = subprocess.run([binary, str(root / 'does-not-exist.md')], capture_output=True)
print('missing file:', r.returncode, repr(r.stderr))
assert r.returncode == 101
print('All observed CLI discrepancies reproduced.')
