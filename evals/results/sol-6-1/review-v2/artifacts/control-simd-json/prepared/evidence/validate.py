from pathlib import Path
import hashlib,json,re,subprocess

root=Path('/work/crate')
prepared=Path('/work/prepared')
evidence=prepared/'evidence'
controls=json.loads((prepared/'controls.json').read_text())
assert set(controls)=={'project','decoys','false_findings'}
assert controls['project']=='simd-json'
assert [c['id'] for c in controls['decoys']]==['decoy-1','decoy-2']
assert [c['id'] for c in controls['false_findings']]==[f'false-{i}' for i in range(1,6)]
all_controls=controls['decoys']+controls['false_findings']
for c in controls['decoys']:
    assert set(c)=={'id','kind','summary','location','match_rule','refutation'}
    assert c['kind']=='wrong-behavior'
    assert isinstance(c['location'],list) and c['location']
for c in controls['false_findings']:
    assert set(c)=={'id','title','location','body','refutation'}
    assert isinstance(c['location'],str)
for c in all_controls:
    ref=c['refutation']
    assert set(ref)=={'method','evidence','commands','source_refs'}
    assert ref['method']=='ran'
    assert ref['evidence'] and ref['commands'] and ref['source_refs']
    for cmd in ref['commands']:
        assert cmd=='/work/prepared/evidence/run-probes.sh'
    locations=c['location'] if isinstance(c['location'],list) else [c['location']]
    for loc in locations+ref['source_refs']:
        filename,n=loc.rsplit(':',1)
        lines=(root/filename).read_text().splitlines()
        assert filename.startswith('src/') and 1<=int(n)<=len(lines),loc
    assert c['id']+': ' in (evidence/(c['id']+'.log')).read_text()
print('PASS: exact schema, two decoys, five separate false findings, required IDs and refutation fields')
print('PASS: all finding locations and refutation source references exist in the original src tree')

manifest=json.loads((evidence/'baseline-sha256.json').read_text())
current={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.rglob('*')) if p.is_file()}
assert current==manifest
print(f'PASS: all {len(manifest)} original crate files unchanged; no files added or removed')
for p in root.joinpath('src').rglob('*'):
    if p.is_file():
        assert p.read_bytes()==(Path('/work/tmp/control-crate/src')/p.relative_to(root/'src')).read_bytes()
for name in ['Cargo.toml','Cargo.lock']:
    assert (root/name).read_bytes()==(Path('/work/tmp/control-crate')/name).read_bytes()
assert (evidence/'negative_controls.rs').read_bytes()==Path('/work/tmp/control-crate/tests/negative_controls.rs').read_bytes()
print('PASS: scratch crate source and Cargo manifest/lock match baseline; preserved probe source matches executed harness')

log=(evidence/'probe-run.log').read_text()
assert 'test result: ok. 7 passed; 0 failed;' in log
assert log.rstrip().endswith('Exit code: 0')
for name in [
    'decoy_1_empty_known_length_sequence',
    'decoy_2_ignored_nested_fields_are_consumed',
    'false_1_u8_range_is_checked',
    'false_2_second_root_is_rejected',
    'false_3_buffers_do_not_keep_old_structural_indexes',
    'false_4_signed_integer_map_keys_deserialize',
    'false_5_array_index_skips_whole_nested_values',
]:
    assert 'test '+name+' ... ' in log
print('PASS: successful log contains all seven independent probe tests, zero failures, exit code 0')
print('Validation command: python3 /work/prepared/evidence/validate.py')
print(subprocess.check_output(['rustc','--version'],text=True).strip())
print(subprocess.check_output(['cargo','--version'],text=True).strip())
print(subprocess.check_output(['uname','-m'],text=True).strip())
