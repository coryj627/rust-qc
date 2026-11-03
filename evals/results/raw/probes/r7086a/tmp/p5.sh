J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; "$@" 2>&1 | head -${LINES_MAX:-12}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
mk; printf 'r:\n  echo hi\n' > justfile; run ts-bad $J --timestamp --timestamp-format '%Q' r
mk; cat > justfile <<'J'
p := """
\t  a
\t  b
"""
q := f"""
\t  a
\t  b
"""
J
for v in p q; do echo "[$v]"; $J --evaluate $v | od -c | head -3; done
mk; cat > justfile <<'J'
v := "  deep\n shallow"
a := f'''
    x
    {{ v }}
'''
b := f'''
    x
    {{ "" }}
'''
c := f'''
    x

    {{ "y" }}
'''
J
for v in a b c; do echo "[$v]"; $J --evaluate $v | od -c | head -4; done
# lazy diamond
mk; python3 - <<'P'
n=26
s="set lazy\n"
for i in range(n):
    s+=f"a{i}: b{i} c{i}\nb{i}: a{i+1}\nc{i}: a{i+1}\n"
s+=f"a{n}:\n  @echo end\n"
open("justfile","w").write(s)
P
echo "--- lazy-diamond"; time (timeout 30 $J a0; echo rc=$?); sed -i 1d justfile; echo "--- nolazy-diamond"; time (timeout 30 $J a0; echo rc=$?)
# large file
mk; python3 - <<'P'
s=""
for i in range(30000):
    s+=f"v{i} := 'x'\nr{i} a='b':\n  echo {{{{a}}}} {{{{v{i}}}}}\n\n"
open("justfile","w").write(s)
P
echo "--- large"; time ($J --list | wc -l); time ($J --quiet r5)
mk; python3 - <<'P'
s=""
n=60000
for i in range(n):
    s+=f"a{i} := a{i+1}\n"
s+=f"a{n} := 'x'\n"
open("justfile","w").write(s)
P
echo "--- var-chain"; $J --list 2>&1 | tail -2; echo rc=$?
mk; python3 - <<'P'
s=""
n=30000
for i in range(n):
    s+=f"r{i}: r{i+1}\n"
s+=f"r{n}:\n  @echo end\n"
open("justfile","w").write(s)
P
echo "--- dep-chain"; $J r0 2>&1 | tail -2
mk; printf '%s\n' 'x := "'"$(python3 -c 'print("a"*5000000)')"'"' 'r:' > justfile; echo "--- long string"; time ($J --evaluate x | wc -c)
mk; printf 'r a:\n  echo "{{a}}"\n' > justfile; run recipe-newline $J r "$(printf 'x\ny')"
mk; printf 'x := "a"\n[confirm("sure {{x}}")]\nr:\n  echo\n[confirm(x + "?")]\ns:\n  echo s\n' > justfile; echo y | $J s; echo rc=$?; run conf-dry sh -c "echo n | $J --dry-run s"
