J=/target/debug/just
T=/work/tmp/p/w; 
mk() { d=$(mktemp -d -p /work/tmp/p); cd $d; cat > justfile; }
echo "--- 1 override in defined fn"
mk <<'J'
set unstable
x := "orig"
f() := x
a:
  @echo {{f()}} {{x}}
J
$J x=new a 2>&1
echo "--- 2 recursion"
mk <<'J'
set unstable
f() := f()
a:
  @echo {{f()}}
J
$J a 2>&1 | tail -5; echo rc=$?
mk <<'J'
set unstable
x := f()
f() := x
a:
  @echo {{x}}
J
$J a 2>&1 | tail -5
echo "--- 3 is_dependency in fn"
mk <<'J'
set unstable
f() := is_dependency()
a: b
b:
  @echo {{f()}} {{is_dependency()}}
J
$J a 2>&1
echo "--- 4 env attr in body backtick"
mk <<'J'
[env('FOO','bar')]
a:
  @echo [{{`echo ${FOO:-unset}`}}] [{{shell('echo ${FOO:-unset}')}}] [$FOO]
J
$J a 2>&1
echo "--- 5 eval shell bang"
mk <<'J'
x := 'a!b'
J
$J --evaluate --evaluate-format shell 2>&1; eval "$($J --evaluate --evaluate-format shell)"; echo "x=[$x]"
echo "--- 6 dotenv path missing"
mk <<'J'
set dotenv-path := "nope.env"
a:
  @echo ${FOO:-unset}
J
echo FOO=fromdotenv > .env
$J a 2>&1
echo "--- 7 bad shebang"
mk <<'J'
a:
  #!
  echo hi
J
$J a 2>&1
mk <<'J'
x := ''
a:
  #!{{x}}
  echo hi
J
$J a 2>&1
echo "--- 8 mixed ws"
printf 'a:\n  echo a\n \techo b\n' > justfile; $J a 2>&1
printf 'a:\n\techo a\n \techo b\n' > justfile; $J a 2>&1
echo "--- 9 lazy"
mk <<'J'
set lazy
x := `echo X; echo ran >&2`
y := 'Y'
z := 'Z'
[confirm(y)]
a p=z: (b x)
  @echo {{p}}
b v:
  @echo {{v}}
c:
  @echo c
J
echo y | $J a 2>&1; $J c 2>&1
echo "--- 10 lazy with submodule/parent var"
mkdir -p m; cat > justfile <<'J'
set lazy
x := 'X'
y := `echo Y; echo ran-y >&2`
mod m
J
cat > m/mod.just <<'J'
v := 'V'
foo:
  @echo {{v}}
J
$J m foo 2>&1 ; $J m::foo 2>&1
J2=1
cat > justfile <<'J'
set lazy
x := 'X'
a: m::foo
mod m
J
cat > m/mod.just <<'J'
set lazy
v := 'V'
foo:
  @echo {{v}}
J
$J a 2>&1
echo "--- 11 eval in module"
cat > justfile <<'J'
x := 'X'
_p := 'P'
mod m
J
cat > m/mod.just <<'J'
v := 'V'
J
$J --evaluate m 2>&1;$J --evaluate m::v 2>&1; echo; $J --evaluate m v 2>&1; echo; $J --evaluate 2>&1; $J --evaluate _p 2>&1; echo
