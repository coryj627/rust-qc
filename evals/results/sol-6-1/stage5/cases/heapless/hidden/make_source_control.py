#!/usr/bin/env python3
"""Create a default-feature Cargo harness around untouched library source.
Control validation only; never use this to bypass candidate manifest changes.
"""
from pathlib import Path
import json
import sys
import tomllib

if len(sys.argv) != 3:
    raise SystemExit('usage: make_source_control.py SOURCE_ROOT OUTPUT_DIRECTORY')
source = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2]).resolve()
if output.exists():
    raise SystemExit(f'output already exists: {output}')
manifest = tomllib.loads((source / 'Cargo.toml').read_text())
output.mkdir(parents=True)
(output / 'tests').mkdir()
feature_names = sorted(manifest['features'])
# These are the actual default-feature runtime dependencies of both snapshots.
text = f'''[package]
name = "heapless"
version = "{manifest['package']['version']}"
edition = "2021"
build = {json.dumps(str(source / 'build.rs'))}

[lib]
path = {json.dumps(str(source / 'src/lib.rs'))}

[dependencies]
hash32 = "0.3.0"
stable_deref_trait = {{ version = "1", default-features = false }}

[dev-dependencies]
critical-section = {{ version = "1.1", features = ["std"] }}
static_assertions = "1.1.0"

[features]
'''
text += ''.join(f'{json.dumps(name)} = []\n' for name in feature_names)
(output / 'Cargo.toml').write_text(text)
for name in ('heldback_truncate_upstream.rs', 'heldback_truncate_spec.rs'):
    (output / 'tests' / name).write_bytes((Path(__file__).parent / name).read_bytes())
print(f'Created {output}; default features only; source unchanged: {source}')
