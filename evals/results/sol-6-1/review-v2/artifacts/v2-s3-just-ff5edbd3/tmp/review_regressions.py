"""Behavioral regressions for just 1.50.0; intentionally fail on this release.
Only temporary justfiles under /work/tmp are changed. No production fixes.
"""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

BINARY = '/target/debug/just'
BASE = Path('/work/tmp/review-regressions')
BASE.mkdir(exist_ok=True)

class ReviewRegressions(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix=self._testMethodName + '-', dir=BASE))

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def invoke(self, *args, cwd=None):
        result = subprocess.run([BINARY, *args], cwd=cwd or self.root,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                text=True, timeout=10)
        return result

    def success(self, *args, cwd=None):
        result = self.invoke(*args, cwd=cwd)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_format_multiline_module_docs_remains_valid(self):
        self.write('justfile', '[doc("first\\nsecond")]\nmod child\n')
        self.write('child.just', 'show:\n')
        self.success('--list')
        self.success('--fmt')
        self.success('--list')

    def test_format_preserves_private_module(self):
        self.write('justfile', '[private]\nmod child\n')
        self.write('child.just', 'show:\n')
        before = self.success('--list', '--list-submodules')
        self.success('--fmt')
        self.assertEqual(self.success('--list', '--list-submodules'), before)

    def test_format_preserves_private_alias(self):
        self.write('justfile', '[private]\nalias hidden := show\nshow:\n')
        before = self.success('--list')
        self.success('--fmt')
        self.assertEqual(self.success('--list'), before)

    def test_format_preserves_eager_assignment(self):
        self.write('justfile', "set lazy\neager x := `echo hit >> hits`\nshow:\n")
        self.success('show')
        self.success('--fmt')
        self.success('show')
        self.assertEqual((self.root / 'hits').read_text(), 'hit\nhit\n')

    def test_child_module_assignment_shadows_parent(self):
        self.write('justfile', "mod child\nexport x := 'parent'\n")
        self.write('child.just', "a := x\nexport x := 'child'\nshow:\n  @echo '{{a}} {{x}}'\n")
        self.assertEqual(self.success('child', 'show'), 'child child\n')

    def test_redefined_constant_is_used_in_other_assignment(self):
        self.write('justfile', "A := HEX\nHEX := 'custom'\n")
        self.assertEqual(self.success('--eval', 'A'), 'custom')

    def test_reused_module_sources_have_distinct_identity(self):
        self.write('justfile', "mod a 'shared.just'\nmod b 'shared.just'\n")
        self.write('shared.just', "show:\n  @echo '{{module_path()}}'\n")
        self.assertEqual(self.success('a', 'show', 'b', 'show'), 'a\nb\n')

    def test_function_uses_overridden_global(self):
        self.write('justfile', "set unstable\nx := 'original'\nf() := x\nshow:\n  @echo '{{x}} {{f()}}'\n")
        self.assertEqual(self.success('x=overridden', 'show'), 'overridden overridden\n')

    def test_function_reuses_evaluated_global(self):
        self.write('justfile', "set unstable\nx := `echo hit >> hits; echo value`\nf() := x\nshow:\n  @echo '{{x}} {{f()}} {{f()}}'\n")
        self.success('show')
        self.assertEqual((self.root / 'hits').read_text(), 'hit\n')

    def test_function_parameter_does_not_change_transitive_global(self):
        self.write('justfile', "set unstable\nx := 'global'\ny := x\nf(x) := y\nshow:\n  @echo '{{f(\"parameter\")}}'\n")
        self.assertEqual(self.success('show'), 'global\n')

    def test_function_preserves_dependency_context(self):
        self.write('justfile', "set unstable\nf() := is_dependency()\nshow: dep\ndep:\n  @echo '{{is_dependency()}} {{f()}}'\n")
        self.assertEqual(self.success('show'), 'true true\n')

    def test_invalid_datetime_formats_return_diagnostics(self):
        for function, fmt in [('datetime', '%Q'), ('datetime_utc', '%')]:
            with self.subTest(function=function, fmt=fmt):
                self.write('justfile', f"x := {function}('{fmt}')\n")
                result = self.invoke('--eval', 'x')
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertNotIn('panicked', result.stderr)

    def test_invalid_timestamp_format_returns_diagnostic(self):
        self.write('justfile', 'show:\n  echo hello\n')
        result = self.invoke('--timestamp', '--timestamp-format', '%Q', 'show')
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertNotIn('panicked', result.stderr)

    def test_ambiguous_external_module_directory_returns_diagnostic(self):
        self.write('project/justfile', "mod child '../shared'\n")
        self.write('shared/justfile', 'show:\n')
        self.write('shared/.justfile', 'show:\n')
        result = self.invoke('--list', cwd=self.root / 'project')
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn('multiple', result.stderr.lower())
        self.assertNotIn('panicked', result.stderr)

    def test_relative_ceiling_stops_parent_search(self):
        self.write('justfile', 'show:\n  @echo parent\n')
        (self.root / 'child').mkdir()
        for ceiling in ['.', str(self.root / 'child' / '..' / 'child')]:
            with self.subTest(ceiling=ceiling):
                result = self.invoke('--ceiling', ceiling, 'show', cwd=self.root / 'child')
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertEqual(result.stdout, '')

if __name__ == '__main__':
    unittest.main(verbosity=2)
