#!/usr/bin/env python3
"""Grading cache names must be isolated while original sandbox arguments survive."""

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


class GradingSandboxTests(unittest.TestCase):
    def invoke(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = Path(__file__).parent.parent / "review-v2/method/sb-namespace"
            if source.exists():
                shutil.copyfile(source, root / "sb")
            original = root / "sb-original"
            original.write_text(
                '#!/usr/bin/env python3\nimport json, os, sys\n'
                'keys = ["RQC_TARGET_VOLUME", "RQC_TIMEOUT", "RQC_MEM", "RQC_CPUS", '
                '"RQC_WAIT_LOG", "RUSTUP_AUTO_INSTALL"]\n'
                'print(json.dumps({"argv": sys.argv[1:], '
                '"environment": {k: os.environ.get(k) for k in keys}}))\n'
            )
            original.chmod(0o755)
            environment = {**os.environ, "RQC_TARGET_VOLUME": "rqc-sol61-key-s3-moka-1",
                           "RQC_TIMEOUT": "123", "RQC_MEM": "2500m", "RQC_CPUS": "4",
                           "RQC_WAIT_LOG": "operator-waits.log", "RUSTUP_AUTO_INSTALL": "0"}
            result = subprocess.run([
                "/bin/sh", str(root / "sb"), "run", "key-s3-moka-1", "printf", "ok",
            ], env=environment, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            return json.loads(result.stdout)

    def test_grading_uses_a_different_cache_from_prior_condition(self):
        result = self.invoke()
        self.assertEqual(result["environment"]["RQC_TARGET_VOLUME"],
                         "rqc-sol61-v2-key-s3-moka-1")

    def test_original_arguments_and_limits_are_preserved(self):
        result = self.invoke()
        self.assertEqual(result["argv"], ["run", "key-s3-moka-1", "printf", "ok"])
        fields = result["environment"]
        fields.pop("RQC_TARGET_VOLUME")
        self.assertEqual(fields, {"RQC_TIMEOUT": "123", "RQC_MEM": "2500m", "RQC_CPUS": "4",
                                  "RQC_WAIT_LOG": "operator-waits.log",
                                  "RUSTUP_AUTO_INSTALL": "0"})


if __name__ == "__main__":
    unittest.main()
