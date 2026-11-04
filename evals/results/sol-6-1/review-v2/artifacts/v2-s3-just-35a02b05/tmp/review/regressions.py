"""Ordinary contract regressions against the unmodified just 1.50.0 binary.
All generated justfiles are confined to /work/tmp/review/regressions/.
"""
from pathlib import Path
import subprocess
import unittest

BINARY = '/target/debug/just'
ROOT = Path('/work/tmp/review/regressions')

class Regressions(unittest.TestCase):
    def fixture(self, files):
        directory = ROOT / self._testMethodName
        directory.mkdir(parents=True, exist_ok=True)
        for name, contents in files.items():
            path = directory / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents)
        return directory

    def invoke(self, directory, *args):
        return subprocess.run([BINARY, *args], cwd=directory,
                              capture_output=True, text=True, timeout=10)

    def test_shared_module_source_keeps_both_identities(self):
        directory = self.fixture({
            'justfile': "mod first 'shared.just'\nmod second 'shared.just'\n",
            'shared.just': 'task:\n  @echo {{module_path()}}\n',
        })
        result = self.invoke(directory, 'first::task', 'second::task')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, 'first\nsecond\n')

    def test_shared_import_override_is_local_to_module(self):
        directory = self.fixture({
            'justfile': 'mod first\nmod second\n',
            'first.just': "import 'common.just'\n",
            'second.just': "import 'common.just'\n",
            'common.just': "x := 'default'\ntask:\n  @echo {{module_path()}} {{x}}\n",
        })
        result = self.invoke(directory, '--set', 'first::x', 'override',
                             'first::task', 'second::task')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, 'first override\nsecond default\n')

    def test_formatter_preserves_private_alias(self):
        directory = self.fixture({
            'justfile': '[private]\nalias internal := task\n\ntask:\n',
        })
        before = self.invoke(directory, '--list')
        self.assertNotIn('internal', before.stdout)
        result = self.invoke(directory, '--fmt')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.invoke(directory, '--list').stdout, before.stdout)

    def test_formatter_preserves_private_module(self):
        directory = self.fixture({
            'justfile': '[private]\nmod helper\n\ntask:\n',
            'helper.just': 'subtask:\n',
        })
        before = self.invoke(directory, '--list')
        self.assertNotIn('helper', before.stdout)
        result = self.invoke(directory, '--fmt')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.invoke(directory, '--list').stdout, before.stdout)

    def test_function_uses_overridden_module_variable(self):
        directory = self.fixture({
            'justfile': "set unstable\nx := 'original'\nf() := x\n"
                        'show:\n  @echo {{x}} {{f()}}\n',
        })
        result = self.invoke(directory, '--set', 'x', 'override', 'show')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, 'override override\n')

    def shell_roundtrip(self, shell):
        directory = self.fixture({'justfile': "greeting := 'Hello!'\n"})
        result = self.invoke(directory, '--evaluate', '--evaluate-format', 'shell')
        self.assertEqual(result.returncode, 0, result.stderr)
        evaluated = subprocess.run(
            [shell, '-c', result.stdout + '\nprintf \'%s\' "$greeting"'],
            cwd=directory, capture_output=True, text=True, timeout=10)
        self.assertEqual(evaluated.returncode, 0, evaluated.stderr)
        self.assertEqual(evaluated.stdout, 'Hello!')

    def test_shell_format_roundtrips_exclamation_in_sh(self):
        self.shell_roundtrip('sh')

    def test_shell_format_roundtrips_exclamation_in_bash(self):
        self.shell_roundtrip('bash')

    def test_relative_ceiling_stops_parent_search(self):
        directory = self.fixture({'justfile': 'task:\n  @echo parent\n',
                                  'child/placeholder': ''})
        result = self.invoke(directory / 'child', '--ceiling', '.')
        self.assertEqual(result.returncode, 1, (result.stdout, result.stderr))
        self.assertEqual(result.stderr, 'error: No justfile found\n')

    def test_cross_module_alias_is_listed(self):
        directory = self.fixture({
            'justfile': 'alias shortcut := helper::task\nmod helper\n',
            'helper.just': 'task:\n  @echo helper\n',
        })
        self.assertEqual(self.invoke(directory, 'shortcut').stdout, 'helper\n')
        result = self.invoke(directory, '--list', '--alias-style', 'separate')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('shortcut', result.stdout)

    def test_datetime_bad_format_reports_error(self):
        directory = self.fixture({'justfile': "x := datetime('%Q')\n"})
        result = self.invoke(directory, '--evaluate', 'x')
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn('error:', result.stderr)

    def test_datetime_utc_bad_format_reports_error(self):
        directory = self.fixture({'justfile': "x := datetime_utc('%')\n"})
        result = self.invoke(directory, '--evaluate', 'x')
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn('error:', result.stderr)

    def test_timestamp_bad_format_reports_error(self):
        directory = self.fixture({'justfile': 'task:\n  @echo done\n'})
        result = self.invoke(directory, '--timestamp', '--timestamp-format', '%Q')
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('panicked', result.stderr)
        self.assertIn('error:', result.stderr)

    def test_control_absolute_ceiling_stops_parent_search(self):
        directory = self.fixture({'justfile': 'task:\n  @echo parent\n',
                                  'child/placeholder': ''})
        result = self.invoke(directory / 'child', '--ceiling', str(directory / 'child'))
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(result.stderr, 'error: No justfile found\n')

    def test_control_plain_variable_override_works(self):
        directory = self.fixture({'justfile': "x := 'original'\n"})
        result = self.invoke(directory, '--set', 'x', 'override', '--evaluate', 'x')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, 'override')

if __name__ == '__main__':
    unittest.main(verbosity=2)
