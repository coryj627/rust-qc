from pathlib import Path
import os
import subprocess
import unittest

BINARY = '/target/debug/pulldown-cmark'
class CliChecks(unittest.TestCase):
    def test_multiple_files(self):
        a, b = Path('/work/tmp/a.md'), Path('/work/tmp/b.md')
        a.write_text('one\n')
        b.write_text('two\n')
        result = subprocess.run([BINARY, str(a), str(b)], capture_output=True)
        print('multiple_files:', result, flush=True)
        self.assertEqual(result.stdout, b'<p>one</p>\n<p>two</p>\n')
    def test_invalid_utf8_stdin(self):
        result = subprocess.run([BINARY], input=b'hello\xffworld', capture_output=True)
        print('invalid_utf8_stdin:', result, flush=True)
        self.assertNotEqual(result.returncode, 0)
    def test_output_error(self):
        read_fd, write_fd = os.pipe()
        os.close(read_fd)
        try:
            result = subprocess.run([BINARY], input=b'hello\n', stdout=write_fd, stderr=subprocess.PIPE)
        finally:
            os.close(write_fd)
        print('output_error:', result, flush=True)
        self.assertNotEqual(result.returncode, 0)
if __name__ == '__main__':
    unittest.main()
