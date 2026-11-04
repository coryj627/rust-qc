from pathlib import Path
import subprocess
import unittest

root = Path('/work/tmp/cli-fixtures')
root.mkdir(exist_ok=True)
exe = '/target/debug/pulldown-cmark'

class CliRegression(unittest.TestCase):
    def test_multiple_files_not_accumulated(self):
        a, b = root / 'one.md', root / 'two.md'
        a.write_text('one\n')
        b.write_text('two\n')
        p = subprocess.run([exe, str(a), str(b)], capture_output=True)
        self.assertEqual(p.returncode, 0)
        self.assertEqual(p.stdout, b'<p>one</p>\n<p>two</p>\n')

    def test_invalid_utf8_stdin_reports_failure(self):
        p = subprocess.run([exe], input=b'hello\xffworld', capture_output=True)
        print('invalid UTF-8:', p.returncode, repr(p.stdout), repr(p.stderr), flush=True)
        self.assertNotEqual(p.returncode, 0)

    def test_closed_stdout_reports_failure(self):
        source = root / 'output.md'
        source.write_text('x' * (2 * 1024 * 1024))
        p = subprocess.Popen([exe, str(source)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        p.stdout.close()
        error = p.stderr.read()
        p.stderr.close()
        result = p.wait()
        print('closed stdout:', result, repr(error), flush=True)
        self.assertNotEqual(result, 0)

unittest.main(verbosity=2)
