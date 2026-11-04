"""Ordinary CLI regression expectations; fixtures and outputs stay in /work/tmp."""
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

BASE = Path('/work/tmp/regression-cases')
BASE.mkdir(exist_ok=True)
BINARY = '/target/debug/just'

class ReviewRegressions(unittest.TestCase):
    def fixture(self, name, source, files=None):
        d = Path(tempfile.mkdtemp(prefix=name+'-', dir=BASE))
        (d/'justfile').write_text(source)
        for name, text in (files or {}).items():
            p = d/name
            p.parent.mkdir(exist_ok=True, parents=True)
            p.write_text(text)
        return d

    def run_just(self, d, *args):
        p = subprocess.run([BINARY, *args], cwd=d, capture_output=True, text=True, timeout=8)
        with (d/'commands.jsonl').open('a') as f:
            f.write(json.dumps({'args':args,'status':p.returncode,'stdout':p.stdout,'stderr':p.stderr})+'\n')
        return p

    def expect(self, d, args, code=0, stdout=None):
        p = self.run_just(d, *args)
        self.assertEqual(p.returncode, code, p.stderr)
        if stdout is not None:
            self.assertEqual(p.stdout, stdout)
        return p

    def test_control_ordinary_dependency(self):
        d = self.fixture('control-dependency', "base := 'value'\nderived := base\n@show: dep\n  echo {{derived}}\n@dep:\n  echo dependency\n")
        self.expect(d, ['show'], stdout='dependency\nvalue\n')

    def test_control_function_parameter(self):
        d = self.fixture('control-function', "set unstable\nf(x) := x + '!'\n@show:\n  echo {{f('hello')}}\n")
        self.expect(d, ['show'], stdout='hello!\n')

    def test_control_shell_unicode(self):
        d = self.fixture('control-shell', "foo := 'hello π'\n")
        p = self.expect(d, ['--evaluate','--evaluate-format','shell'])
        q = subprocess.run(['sh','-c',p.stdout+'printf "%s" "$foo"'], capture_output=True,text=True,timeout=8)
        self.assertEqual((q.returncode,q.stdout),(0,'hello π'))

    def test_formatter_preserves_multiline_module_documentation(self):
        d = self.fixture('format-module-doc', '[doc("first\\nsecond")]\nmod foo\n', {'foo.just':'show:\n'})
        self.expect(d, ['--list'])
        self.expect(d, ['--fmt'])
        self.expect(d, ['--list'])

    def test_formatter_preserves_eager(self):
        d = self.fixture('format-eager', 'set unstable\nset lazy\neager check := `exit 9`\ndefault:\n')
        self.expect(d, [], code=9)
        self.expect(d, ['--fmt'])
        self.expect(d, [], code=9)

    def test_formatter_preserves_private_module(self):
        d = self.fixture('format-private-module', '[private]\nmod hidden\nvisible:\n', {'hidden.just':'secret:\n'})
        before = self.expect(d, ['--list']).stdout
        self.expect(d, ['--fmt'])
        self.expect(d, ['--list'], stdout=before)

    def test_formatter_preserves_private_alias(self):
        d = self.fixture('format-private-alias', '[private]\nalias hidden := target\ntarget:\n')
        before = self.expect(d, ['--list']).stdout
        self.expect(d, ['--fmt'])
        self.expect(d, ['--list'], stdout=before)

    def test_module_instances_have_distinct_identity(self):
        d = self.fixture('module-identity', "mod a 'shared.just'\nmod b 'shared.just'\n", {'shared.just':"@show:\n  echo {{module_path()}}\n"})
        self.expect(d, ['a::show','b::show'], stdout='a\nb\n')

    def test_constant_redefinition_applies_to_references(self):
        d = self.fixture('constant-reference', "A := HEX\nHEX := 'custom'\n@show:\n  echo {{A}} {{HEX}}\n")
        self.expect(d, ['show'], stdout='custom custom\n')

    def test_constant_redefinition_applies_to_selective_evaluation(self):
        d = self.fixture('constant-evaluate', "HEX := 'custom'\nz := HEX\n")
        self.expect(d, ['--evaluate','z'], stdout='custom')

    def test_module_lazy_is_respected(self):
        d = self.fixture('module-lazy', 'mod sub\n', {'sub.just':'set lazy\nunused := `exit 9`\n@show:\n  echo hello\n'})
        self.expect(d, ['sub::show'], stdout='hello\n')

    def test_root_lazy_does_not_override_module_default(self):
        d = self.fixture('module-eager-default', 'set lazy\nmod sub\n', {'sub.just':'unused := `exit 9`\n@show:\n  echo hello\n'})
        self.expect(d, ['sub::show'], code=9)

    def test_unused_assertion_is_not_evaluated_in_lazy_mode(self):
        d = self.fixture('unused-assert-lazy', "set lazy\nunused := assert('a' == 'b', 'unused')\n@show:\n  echo hello\n")
        self.expect(d, ['show'], stdout='hello\n')

    def test_unused_assertion_is_not_evaluated_with_evaluate(self):
        d = self.fixture('unused-assert-evaluate', "good := 'ok'\nbad := assert('a' == 'b', 'unused')\n")
        self.expect(d, ['--evaluate','good'], stdout='ok')

    def test_function_observes_variable_override(self):
        d = self.fixture('function-override', "set unstable\nx := 'original'\nf() := x\n@show:\n  echo {{x}} {{f()}}\n")
        self.expect(d, ['x=override','show'], stdout='override override\n')

    def test_function_reuses_evaluated_assignment(self):
        d = self.fixture('function-cache', "set unstable\nx := `echo run >> count; echo value`\nf() := x\n@show:\n  echo {{x}} {{f()}} {{f()}}\n")
        self.expect(d, ['show'], stdout='value value value\n')
        self.assertEqual((d/'count').read_text(), 'run\n')

    def test_invalid_date_format_returns_error(self):
        for name, source, args in [
            ('datetime', "x := datetime('%Q')\n", ['--evaluate','x']),
            ('datetime-utc', "x := datetime_utc('%')\n", ['--evaluate','x']),
            ('timestamp', '@show:\n  echo hello\n', ['--timestamp','--timestamp-format','%Q','show']),
        ]:
            with self.subTest(name=name):
                d = self.fixture('invalid-format-'+name, source)
                self.expect(d, args, code=1)

    def test_shell_format_preserves_exclamation_mark(self):
        d = self.fixture('shell-exclamation', "foo := 'hello!world'\n")
        p = self.expect(d, ['--evaluate','--evaluate-format','shell'])
        for shell in ['sh','bash']:
            with self.subTest(shell=shell):
                q = subprocess.run([shell,'-c',p.stdout+'printf "%s" "$foo"'],capture_output=True,text=True,timeout=8)
                self.assertEqual((q.returncode,q.stdout),(0,'hello!world'))

if __name__ == '__main__':
    unittest.main(verbosity=2)
