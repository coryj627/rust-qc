import pathlib
import subprocess

root = pathlib.Path('/work/tmp')
first = root / 'one.md'
second = root / 'two.md'
first.write_text('Alpha\n')
second.write_text('Beta\n')
exe = '/target/debug/pulldown-cmark'
checks = []
p = subprocess.run([exe, str(first), str(second)], capture_output=True)
checks.append(('multiple files rendered once each', p.returncode == 0 and p.stdout == b'<p>Alpha</p>\n<p>Beta</p>\n'))
print('multiple files:', p.returncode, repr(p.stdout))
p = subprocess.run([exe], input=b'\xff', capture_output=True)
checks.append(('invalid UTF-8 stdin returns failure', p.returncode != 0))
print('invalid stdin:', p.returncode, repr(p.stdout), repr(p.stderr))
with open('/dev/full', 'wb') as sink:
    p = subprocess.run([exe, str(first)], stdout=sink, stderr=subprocess.PIPE)
checks.append(('output failure returns failure', p.returncode != 0))
print('failed output:', p.returncode, repr(p.stderr))
p = subprocess.run([exe, str(root / 'nonexistent.md')], capture_output=True)
checks.append(('missing input reports error without panic', p.returncode != 0 and b'panicked' not in p.stderr))
print('missing file:', p.returncode, repr(p.stderr))
for label, passed in checks:
    print(('PASS' if passed else 'FAIL') + ': ' + label)
raise SystemExit(0 if all(passed for _, passed in checks) else 1)
