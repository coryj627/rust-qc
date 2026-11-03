J=/target/debug/just
T=/tmp/p; mkdir -p $T; cd $T
run() { echo "--- $1"; shift; timeout 10 "$@" </dev/null 2>&1 | head -${LINES_MAX:-10}; echo "rc=${PIPESTATUS[0]}"; }
mk() { rm -rf $T/w; mkdir -p $T/w; cd $T/w; }
# json determinism
mk; printf 'unexport A\nunexport B\nunexport C\nunexport D\nr:\n' > justfile; for i in 1 2 3 4 5 6; do $J --dump --dump-format json | python3 -c 'import json,sys; print(json.load(sys.stdin)["unexports"])'; done
# shell function args
mk; printf 'set shell := ["bash", "-c"]\nr:\n  echo {{ shell("echo $0 $1", "a") }}\ns:\n  echo {{ shell("echo $1", "a") }}\n' > justfile; run shell-fn $J r; run shell-fn2 $J s
mk; printf 'set shell := ["python3", "-c"]\nr:\n  print(1)\nb := `print(2)`\ns:\n  print({{ b }})\n' > justfile; run py-shell $J r
mk; printf 'set shell := ["sh", "-c"]\nset positional-arguments\nr a b:\n  echo "0=$0 1=$1 2=$2"\n' > justfile; run pos $J r x y
mk; printf 'set shell := ["sh", "-c"]\nr:\n  echo "hello" | cat\n' > justfile; run shell-arg-only $J --shell-arg -cu r; run shell-only $J --shell bash r; run clear $J --clear-shell-args --shell-arg -c r;
mk; printf 'set shell := ["python3", "-c"]\nr:\n  print(1)\n' > justfile; run shell-arg-dropped $J --shell-arg -c r
# dotenv specifics
mk; printf 'A=1\nB=$A-x\nC="q r"\nD='"'"'s t'"'"'\nexport E=5\n# cmt\nF\n' > .env; printf 'set dotenv-load\nr:\n  echo "$A $B $C $D $E"\n' > justfile; run dotenv-parse $J r
mk; printf 'A=1\n' > .env; printf 'set dotenv-load\nA := "var"\nr:\n  echo $A {{ env("A") }} {{A}}\n' > justfile; A=fromenv run dotenv-env-prec $J r; run dotenv-noenv $J r
mk; printf 'A=dot\n' > .env; printf 'set dotenv-load\nset dotenv-override\nexport A := "exp"\nr:\n  echo $A\nx := `echo $A`\ns:\n  echo {{x}}\n' > justfile; A=fromenv run dotenv-override $J r; A=fromenv run dotenv-override2 $J s;
mk; mkdir m; printf 'A=root\nB=root\n' > .env; printf 'A=mod\n' > m/.env; printf 'set dotenv-load\nmod m\n' > justfile; printf 'set dotenv-load\nr:\n  echo $A $B\n' > m/mod.just; run dotenv-mod $J m::r
mk; printf 'set dotenv-load\nr:\n  echo\n' > justfile; printf 'FOO=\xff\xfe\n' > .env; run dotenv-nonutf8 $J r; mkdir sub; chmod 000 .env; run dotenv-noperm $J r
# env var function non-utf8
mk; printf 'r:\n  echo {{ env("FOO", "dflt") }}\n' > justfile; FOO=$(printf '\xff') run env-nonutf8 $J r
mk; printf 'x := env_var("BAD=NAME")\ny := env("")\nz := env("A\\u{0}B", "d")\nr:\n  echo {{x}}\n' > justfile; run env-badname $J r
mk; printf 'z := env("A\\u{0}B", "d")\nr:\n  echo {{z}}\n' > justfile; run env-nul $J r
mk; printf 'r a="x\\u{0}y":\n  echo {{a}}\n' > justfile; run nul-cmd $J r
mk; printf 'export Z := "x\\u{0}y"\nr:\n  echo hi\n' > justfile; run nul-env $J r
mk; printf 'export A\\u{3d} := "x"\n' > justfile; run eq-env $J --list
mk; printf 'set shell := ["sh\\u{0}", "-c"]\nr:\n  echo hi\n' > justfile; run nul-shell $J r
mk; printf 'r:\n  echo {{ path_exists("a\\u{0}b") }} {{ canonicalize(".") }} {{ read("a\\u{0}") }}\n' > justfile; run nul-path $J r
mk; printf 'r:\n  echo {{ sha256_file("a\\u{0}") }}\n' > justfile; run nul-sha $J r
mk; printf 'r:\n  echo {{ blake3_file("nonexist") }}\n' > justfile; run nofile-blake $J r; mkdir d; printf 'r:\n  echo {{ blake3_file("d") }}\n' > justfile; run dir-blake $J r; printf 'r:\n  echo {{ sha256_file("d") }}\n' > justfile; run dir-sha $J r
touch empty; printf 'r:\n  echo {{ blake3_file("empty") }} {{ blake3("") }}\n' > justfile; run empty-blake $J r
mk; printf 'r:\n  echo {{ choose("0","") }}|{{ choose("99999999999999999999","a") }}\n' > justfile; run choose-big $J r
mk; printf 'r:\n  echo {{ choose("3","") }}\n' > justfile; run choose-empty $J r
mk; printf 'r:\n  echo {{ choose("0","") }}|\n' > justfile; run choose-zero $J r
mk; printf 'r:\n  echo {{ replace_regex("abc","(","x") }}\n' > justfile; run re-bad $J r
mk; printf 'x := if "a" =~ "(" { "1" } else { "2" }\nr:\n' > justfile; run re-bad2 $J --list
mk; printf 'r:\n  echo {{ extension("a.tar.gz") }} {{ file_stem("a.tar.gz") }} {{ without_extension("/a/b.c") }} {{ parent_directory("/") }}\n' > justfile; run paths $J r
mk; printf 'r:\n  echo [{{ without_extension("a/") }}] [{{ without_extension("a/..") }}] [{{clean("a/../..")}}] [{{clean("")}}] [{{ file_name("a/") }}] [{{ join("a","/b") }}] \n' > justfile; run paths2 $J r
mk; printf 'r:\n  echo [{{ append("/x", "a  b") }}] [{{ prepend("x/", "") }}] [{{ quote("a'"'"'b") }}] [{{ kebabcase("fooBar baz") }}] [{{ titlecase("foo-bar") }}] [{{ capitalize("éa") }}] [{{ trim_end_match("aXX","X") }}] [{{ trim_start_matches("xxa","x") }}] [{{ encode_uri_component("a b/é") }}] [{{ semver_matches("1.0.0",">=1") }}] [{{uuid()}}]\n' > justfile; run funcs $J -q r; run funcs2 $J r
mk; printf 'r:\n  echo {{ semver_matches("x","1") }}\n' > justfile; run semver $J r
mk; mkdir -p a/b; printf 'mod a\nr:\n' > justfile; mkdir -p a; printf 'mod b\nd := source_directory()\ne := module_directory()\nf := justfile_directory()\ng := source_file()\nh := module_file()\ni := justfile()\nj := invocation_directory()\nz:\n  @echo {{d}} {{e}} {{f}} {{g}} {{h}} {{i}} {{j}}\n' > a/mod.just; printf 'k := source_file()\nz:\n  @echo {{k}} {{module_path()}} {{module_file()}} {{ module_directory() }}\n' > a/b/mod.just; run src-fns $J a::z;run src-fns2 $J a::b::z; cd a/b; run src-fns3 $J a::b::z; cd ../..
