#!/bin/bash
. /work/tmp/lib.sh
R=/work/tmp/rel/release/just
t "release: --show space, dep args"
printf 'r:\n  echo hi\n' > justfile; $R --show ' ' 2>&1 | head -3; echo c=${PIPESTATUS[0]}
cat > justfile <<'E'
[arg('y', long)]
a x='d' y:
  echo {{x}} {{y}}
b: (a '1')
c:
  echo c
E
$R b 2>&1|head -3; echo c=${PIPESTATUS[0]}; $R c 2>&1|head -3; echo c=${PIPESTATUS[0]}; $R --list 2>&1|head -5; echo c=${PIPESTATUS[0]}
t "release chains"
for N in 1000 5000 10000 20000 50000; do python3 -c "print('x := ' + ' + '.join([\"'a'\"]*$N)); print('r:\n  echo')" > justfile; $R --list >/dev/null 2>&1; echo "plus $N c=$?"; done
for N in 500 1000 2000 5000; do python3 -c "print('x := ' + ' + '.join([\"'a'\"]*$N)); print('r:\n  echo')" > justfile; $J --list >/dev/null 2>&1; echo "debug plus $N c=$?"; done
for N in 2000 5000 20000 ; do python3 -c "
N=$N
for i in range(N): print(f'r{i}: r{i+1}')
print(f'r{N}:')" > justfile; $R --list >/dev/null 2>&1; echo "recipes $N c=$?";  python3 -c "
N=$N
for i in range(N): print(f'a{i} := a{i+1}')
print(f'a{N} := \"x\"')" > justfile; $R --list >/dev/null 2>&1; echo "assigns $N c=$?"; done
t "recursive release"
printf 'set unstable\nf(a) := f(a)\nx := f("a")\nr:\n  echo {{x}}\n' > justfile; $R r 2>&1|tail -1; echo c=${PIPESTATUS[0]}
t "guard signal"
cat > justfile <<'E'
set guards
a:
  ?trap '' TERM; kill -TERM $PPID; sleep 0.2; exit 1
  echo not-after-guard
b: a
  echo after-r
E
rr b
sed -n 375,395p /work/crate/src/recipe.rs
t "editor signal"
cat > ed.sh <<'E'
#!/bin/sh
sleep 2; touch ed-done
E
chmod +x ed.sh; printf 'a:\n' > justfile
VISUAL=$D/ed.sh $J --edit </dev/null & P=$!; sleep 0.5; kill -INT $P; wait $P; echo "just exit=$? editor-done-yet=$(ls ed-done 2>/dev/null)"; sleep 2.2; echo "later: $(ls ed-done 2>/dev/null)"; rm -f ed-done
VISUAL=$D/ed.sh $J --edit </dev/null & P=$!; sleep 0.5; kill -TERM $P; wait $P; echo "TERM: just exit=$? editor-done-yet=$(ls ed-done 2>/dev/null)"; sleep 2.2; rm -f ed-done
cat > ch.sh <<'E'
#!/bin/sh
cat >/dev/null; sleep 2; touch ch-done
E
chmod +x ch.sh
$J --choose --chooser $D/ch.sh </dev/null & P=$!; sleep 0.5; kill -TERM $P; wait $P; echo "chooser TERM: just exit=$? done-yet=$(ls ch-done 2>/dev/null)"; sleep 2.2; echo "later: $(ls ch-done 2>/dev/null)"
t "no-cd and setting backtick"
mkdir -p sub w; cat > justfile <<'E'
[no-cd]
s:
  echo {{`pwd`}} $(pwd)
E
cd sub; rr --justfile ../justfile --no-deps s; $J s 2>&1 </dev/null; cd ..
cat > justfile <<'E'
set working-directory := 'w'
x := `pwd`
r:
  echo {{x}} {{`pwd`}} $(pwd)  {{shell('pwd')}} {{absolute_path('.')}}
E
rr r
t "group list vs choose"
printf '[group("g")]\nmod q\na:\n' > justfile; echo 'qr:' > q.just; rr --list --group g;  rr --list; rr --group g --summary
t "_x fmt idempotent"
printf "_x := 'v'\nfoo:\n    echo hi\n" > justfile; rr --unstable --fmt; cat justfile; rr --unstable --fmt --check; rr --list; rr --variables
t "fmt export/other"
printf 'x := x"$HOME/a"\nr:\n    echo {{x}}\n' > justfile; rr --unstable --fmt --check; rr --dump
t "fuzz cfg"
cd /work/crate; RUSTFLAGS="--cfg fuzzing" CARGO_TARGET_DIR=/work/tmp/fz cargo check --lib --locked 2>&1 | tail -12
ls /work/crate/fuzz /work/crate/fuzz/fuzz_targets; cat /work/crate/fuzz/fuzz_targets/*.rs | head -20
