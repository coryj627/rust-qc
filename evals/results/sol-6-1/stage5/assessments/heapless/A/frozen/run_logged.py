#!/usr/bin/env python3
"""Run one offline Cargo command and retain command, output and exit status."""
from pathlib import Path
import os
import shlex
import subprocess
import sys
if len(sys.argv) < 6:
    raise SystemExit('usage: run_logged.py LOG CWD TARGET_DIR COMMAND [ARG...]')
log, cwd, target, *cmd = sys.argv[1:]
env = dict(os.environ, CARGO_HOME='/work/.cargo-home', CARGO_TARGET_DIR=target,
           CARGO_NET_OFFLINE='true', RUSTUP_AUTO_INSTALL='0', CARGO_TERM_COLOR='never')
r = subprocess.run(cmd, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
report = (f'cwd: {cwd}\n'
          f'env: CARGO_HOME=/work/.cargo-home CARGO_TARGET_DIR={target} CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 CARGO_TERM_COLOR=never\n'
          f'$ {shlex.join(cmd)}\n{r.stdout}\nexit_status: {r.returncode}\n')
Path(log).parent.mkdir(parents=True, exist_ok=True)
Path(log).write_text(report)
print(report)
sys.exit(r.returncode)
