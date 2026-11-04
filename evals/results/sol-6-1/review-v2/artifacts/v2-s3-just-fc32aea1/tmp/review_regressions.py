"""Ordinary CLI regression checks. Fixtures live only under /work/tmp."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

BINARY = '/target/debug/just'
RESULTS = []

class Review(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix='review-', dir='/work/tmp')
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)

    def write(self, name, text):
        p = self.root / name
        p.parent.mkdir(exist_ok=True, parents=True)
        p.write_text(text)

    def run_just(self, *args, cwd=None):
        r = subprocess.run([BINARY, *args], cwd=cwd or self.root,
                           stdin=subprocess.DEVNULL, capture_output=True,
                           text=True, timeout=5,
                           env=os.environ | {'RUSTUP_AUTO_INSTALL': '0'})
        RESULTS.append({'test': self.id(), 'args': args, 'code': r.returncode,
                        'stdout': r.stdout, 'stderr': r.stderr})
        return r

    def test_format_multiline_module_doc(self):
        self.write('justfile', '[doc("line 1\\nline 2")]\nmod foo\n')
        self.write('foo.just', 'show:\n')
        self.assertEqual(self.run_just('--list').returncode, 0)
        self.assertEqual(self.run_just('--fmt').returncode, 0)
        self.assertEqual(self.run_just('--list').returncode, 0,
                         (self.root / 'justfile').read_text())

    def test_format_eager(self):
        self.write('justfile', 'set lazy\neager check := `exit 42`\nshow:\n  @echo OK\n')
        self.assertEqual(self.run_just('show').returncode, 42)
        self.assertEqual(self.run_just('--fmt').returncode, 0)
        self.assertEqual(self.run_just('show').returncode, 42)

    def test_format_private_module(self):
        self.write('justfile', '[private]\nmod foo\nshow:\n')
        self.write('foo.just', 'hidden:\n')
        before = self.run_just('--list')
        self.assertEqual(self.run_just('--fmt').returncode, 0)
        self.assertEqual(self.run_just('--list').stdout, before.stdout)

    def test_format_private_alias(self):
        self.write('justfile', '[private]\nalias hidden := show\nshow:\n')
        before = self.run_just('--list')
        self.assertEqual(self.run_just('--fmt').returncode, 0)
        self.assertEqual(self.run_just('--list').stdout, before.stdout)

    def test_function_cycle_reports_error(self):
        self.write('justfile', 'set unstable\nf() := a\na := f()\n')
        r = self.run_just('--evaluate', 'a')
        self.assertEqual(r.returncode, 1, r.stderr)

    def test_function_override(self):
        self.write('justfile', "set unstable\nx := 'original'\nf() := x\nshow:\n  @echo {{x}} {{f()}}\n")
        r = self.run_just('x=override', 'show')
        self.assertEqual((r.returncode, r.stdout), (0, 'override override\n'))

    def test_function_preserves_module_binding(self):
        self.write('justfile', "set unstable\nx := uuid()\nf() := x\nshow:\n  @echo {{x}} {{f()}} {{f()}}\n")
        r = self.run_just('show')
        self.assertEqual(r.returncode, 0)
        values = r.stdout.split()
        self.assertEqual(len(values), 3)
        self.assertEqual(values, [values[0]] * 3)

    def test_function_exports(self):
        self.write('justfile', "set unstable\nexport X := 'value'\nf() := shell('printf %s ${X-missing}')\nshow:\n  @echo {{f()}}\n")
        r = self.run_just('show')
        self.assertEqual((r.returncode, r.stdout), (0, 'value\n'))

    def test_function_dependency(self):
        self.write('justfile', 'set unstable\nf() := is_dependency()\ndep:\n  @echo {{is_dependency()}} {{f()}}\nshow: dep\n')
        r = self.run_just('show')
        self.assertEqual((r.returncode, r.stdout), (0, 'true true\n'))

    def test_reused_module_source(self):
        self.write('justfile', "mod a 'shared.just'\nmod b 'shared.just'\n")
        self.write('shared.just', 'show:\n  @echo {{module_path()}}\n')
        r = self.run_just('a::show', 'b::show')
        self.assertEqual((r.returncode, r.stdout), (0, 'a\nb\n'))

    def test_shell_format_bang_roundtrip(self):
        self.write('justfile', "foo := 'hello!'\n")
        r = self.run_just('--evaluate', '--evaluate-format', 'shell')
        self.assertEqual(r.returncode, 0)
        self.write('values.sh', r.stdout)
        r = subprocess.run(['/bin/sh', '-c', '. ./values.sh; printf "%s" "$foo"'],
                           cwd=self.root, stdin=subprocess.DEVNULL,
                           capture_output=True, text=True, timeout=5)
        RESULTS.append({'test': self.id(), 'sourced_value': r.stdout})
        self.assertEqual((r.returncode, r.stdout), (0, 'hello!'))

    def test_invalid_datetime_format(self):
        self.write('justfile', "x := datetime('%Q')\n")
        self.assertEqual(self.run_just('--evaluate', 'x').returncode, 1)

    def test_invalid_datetime_utc_format(self):
        self.write('justfile', "x := datetime_utc('%')\n")
        self.assertEqual(self.run_just('--evaluate', 'x').returncode, 1)

    def test_invalid_timestamp_format(self):
        self.write('justfile', 'show:\n  @echo OK\n')
        self.assertEqual(self.run_just('--timestamp', '--timestamp-format', '%Q', 'show').returncode, 1)

    def test_cross_module_alias_listing(self):
        self.write('justfile', 'mod foo\nalias quick := foo::build\n')
        self.write('foo.just', "build target='module':\n  @echo {{target}}\n")
        r = self.run_just('--list', '--alias-style', 'separate')
        self.assertEqual(r.returncode, 0)
        self.assertIn('quick', r.stdout)

    def test_ceiling_relative(self):
        self.write('justfile', 'show:\n  @echo PARENT\n')
        sub = self.root / 'sub'
        sub.mkdir()
        absolute = self.run_just('--ceiling', str(sub), 'show', cwd=sub)
        relative = self.run_just('--ceiling', '.', 'show', cwd=sub)
        self.assertEqual(absolute.returncode, 1)
        self.assertEqual((relative.returncode, relative.stdout),
                         (absolute.returncode, absolute.stdout))

    def test_control_plain_format_preserves_behavior(self):
        self.write('justfile', "x:='café'\nshow:\n  @echo {{x}}\n")
        self.assertEqual(self.run_just('show').stdout, 'café\n')
        self.assertEqual(self.run_just('--fmt').returncode, 0)
        self.assertEqual(self.run_just('show').stdout, 'café\n')

    def test_control_distinct_module_sources(self):
        self.write('justfile', 'mod a\nmod b\n')
        for name in ['a', 'b']:
            self.write(name + '.just', 'show:\n  @echo {{module_path()}}\n')
        self.assertEqual(self.run_just('a::show', 'b::show').stdout, 'a\nb\n')

if __name__ == '__main__':
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Review)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    Path('/work/tmp/regression-results.json').write_text(json.dumps(RESULTS, indent=2))
    raise SystemExit(not result.wasSuccessful())
