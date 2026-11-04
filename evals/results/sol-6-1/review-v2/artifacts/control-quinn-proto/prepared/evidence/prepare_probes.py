"""Append test-only modules to a scratch workspace; never alter baseline files."""
import pathlib
import shutil

baseline = pathlib.Path('/work/crate')
scratch = pathlib.Path('/work/tmp/control-probe')
evidence = pathlib.Path('/work/prepared/evidence')
if not scratch.exists():
    shutil.copytree(baseline, scratch)
modules = {
    'src/tests/mod.rs': 'public_api.rs',
    'src/varint.rs': 'varint.rs',
    'src/cid_queue.rs': 'cid_queue.rs',
    'src/connection/send_buffer.rs': 'send_buffer.rs',
    'src/connection/assembler.rs': 'assembler.rs',
    'src/connection/mod.rs': 'idle_timeout.rs',
}
for relative, probe in modules.items():
    destination = scratch / 'quinn-proto' / relative
    destination.write_text((baseline / 'quinn-proto' / relative).read_text() +
        '\n#[cfg(test)]\n#[path = "' + str(evidence / 'probes' / probe) +
        '"]\nmod negative_control_probes;\n')
print('Prepared seven regression probes in scratch workspace:', scratch)
