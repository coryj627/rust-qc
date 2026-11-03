# Times `just --dry-run r0` on a chain of N recipes where each depends on the next one twice.
import os, subprocess, sys, tempfile, time

just = sys.argv[1]
SIZES = [20, 22, 24]


def write(directory, n, lazy):
    text = "set lazy\n" if lazy else ""
    for i in range(n):
        text += f"r{i}: r{i + 1} r{i + 1}\n"
    text += f"r{n}:\n"
    with open(os.path.join(directory, "justfile"), "w") as f:
        f.write(text)


def measure(n, lazy):
    with tempfile.TemporaryDirectory() as directory:
        write(directory, n, lazy)
        start = time.monotonic()
        result = subprocess.run(
            [just, "--dry-run", "r0"], cwd=directory, capture_output=True, timeout=240
        )
        elapsed = time.monotonic() - start
        if result.returncode != 0:
            print(result.stderr.decode()[:400])
            sys.exit(1)
        return elapsed


times = {}
for n in SIZES:
    times[n] = measure(n, True)
    print(f"set lazy    N={n:3d}  {times[n]:8.3f}s", flush=True)

control = measure(SIZES[-1], False)
print(f"no set lazy N={SIZES[-1]:3d}  {control:8.3f}s", flush=True)
ratios = [times[b] / times[a] for a, b in zip(SIZES, SIZES[1:])]
print("ratio per +2 recipes:", " ".join(f"{r:.2f}" for r in ratios), "(2^N predicts 4)")
if all(r > 2.5 for r in ratios) and times[SIZES[-1]] > 0.5 and times[SIZES[-1]] > 5 * control:
    print("DEMONSTRATED: time grows about 4x per 2 added recipes, only under set lazy")
    sys.exit(0)
sys.exit(1)
