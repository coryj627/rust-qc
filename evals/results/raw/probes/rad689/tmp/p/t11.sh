J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 dep on recipe with required option after default positional"
mk <<'J'
[arg('y', long)]
a x='d' y:
  @echo x={{x}} y={{y}}
b: (a '1')
[arg('z', long, value='on')]
c z='off' p='p':
  @echo z={{z}} p={{p}}
e: (c 'bogus')
[arg('q', long, pattern='[0-9]+')]
f q='1' r='r':
  @echo q={{q}} r={{r}}
g: (f 'x')
J
$J b 2>&1 | head -4; $J e 2>&1 | head -3; $J g 2>&1 | head -3
echo "--- 2 --command exit code"
mk <<'J'
a:
J
$J --command sh -c 'exit 3' 2>&1; echo rc=$?; $J --command sh -c 'kill -9 $$' 2>&1; echo rc=$?
echo "--- 3 parallel shebang ETXTBSY"
python3 - <<'P'
names=[f"r{i}" for i in range(16)]
s="[parallel]\nall: "+" ".join(names)+"\n"
for n in names:
  s+=f"{n}:\n  #!/bin/sh\n  true\n"
open('justfile','w').write(s)
P
fails=0; for i in $(seq 1 150); do $J all >out 2>&1 || { fails=$((fails+1)); tail -1 out; }; done; echo fails=$fails
