"""Copy a candidate/control into /work/tmp and add held-back tests only."""
from pathlib import Path
import shutil
import sys

if len(sys.argv) != 3:
    raise SystemExit('usage: python3 stage.py SOURCE DESTINATION_UNDER_WORK_TMP')
source = Path(sys.argv[1]).resolve()
destination = Path(sys.argv[2]).resolve()
if not source.is_relative_to(Path('/work')) or not (source/'Cargo.toml').is_file():
    raise SystemExit('source must be a Rust project under /work')
if not destination.is_relative_to(Path('/work/tmp')) or destination == Path('/work/tmp'):
    raise SystemExit('destination must be a child of /work/tmp')
if destination.exists():
    raise SystemExit('destination exists; choose a fresh destination')
if destination.is_relative_to(source):
    raise SystemExit('destination must not be inside source')
shutil.copytree(source, destination, symlinks=True,
                ignore=shutil.ignore_patterns('target', '.git'))
hidden = Path(__file__).resolve().parent
modules = ('hidden_just_6', 'hidden_just_6_neighbors')
lib = destination/'tests/lib.rs'
text = lib.read_text()
for module in modules:
    target = destination/f'tests/{module}.rs'
    if target.exists() or f'mod {module};' in text:
        raise SystemExit(f'reserved hidden-test module already exists: {module}')
    shutil.copyfile(hidden/f'{module}.rs', target)
    text += f'\nmod {module};\n'
lib.write_text(text)
print(f'Staged {source} at {destination}; only two test modules and their registrations were added.')
