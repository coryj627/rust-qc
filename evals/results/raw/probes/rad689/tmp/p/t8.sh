J=/target/debug/just
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 unterminated format string continuation"
mk <<'J'
x := f'{{'a'}} tail
J
$J 2>&1 | head -6
printf "x := f'{{'a'}}" > justfile; $J 2>&1 | head -6
printf 'x := f"""{{"a"}} tail\n' > justfile; $J 2>&1 | head -6
printf "x := f'unterminated\n" > justfile; $J 2>&1 | head -6
echo "--- 2 unexports json order"
mk <<'J'
unexport A
unexport B
unexport C
unexport D
J
for i in 1 2 3 4 5 6; do $J --json | python3 -c "import json,sys; print(json.load(sys.stdin)['unexports'])"; done
