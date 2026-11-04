"""Register held-back test modules in an isolated source copy; never edit production."""
from pathlib import Path
import shutil
import sys

root = Path(sys.argv[1]).resolve()
if root in (Path('/work/crate'), Path('/work/fixed')):
    raise SystemExit('Apply only to an isolated copy, not the supplied snapshots.')
hidden = Path(__file__).resolve().parent
lib = root / 'tests/lib.rs'
text = lib.read_text()
for name in ('case_just_6_upstream', 'case_just_6_neighbors'):
    declaration = f'mod {name};'
    target = root / 'tests' / f'{name}.rs'
    if target.exists() and target.read_bytes() != (hidden / target.name).read_bytes():
        raise SystemExit(f'Refusing to overwrite a different test file: {target}')
    shutil.copyfile(hidden / target.name, target)
    if declaration not in text:
        text += '\n' + declaration + '\n'
lib.write_text(text)
print(f'Installed two hidden test modules in {root}; production files unchanged.')
