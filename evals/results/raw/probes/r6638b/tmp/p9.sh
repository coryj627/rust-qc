#!/bin/bash
J=/target/debug/just
cd /work/crate && cargo build --locked --release --bin just 2>&1 | tail -1
d=$(mktemp -d /work/tmp/p/XXXXXX); cd $d
printf '#!/usr/bin/env just --justfile\nfoo:\n  echo hi\n' > justfile; $J --fmt 2>&1; head -2 justfile
printf '## Section\n#indented  doc\n#    aligned\nfoo:\n  echo hi\n' > justfile; $J --fmt 2>&1; head -4 justfile
for B in /target/debug/just /target/release/just; do
 for n in 200 500 1000 2000 5000 10000 20000 50000 100000; do python3 -c "print('x := ' + ' + '.join([\"'a'\"]*$n) + '\nr:\n  @echo hi\n')" > deep.just; timeout 20 $B -f deep.just r >/dev/null 2>err; echo "$B n=$n exit=$? $(head -c 60 err | tr '\n' ' ')"; done
done
for n in 200 255 256 300; do python3 -c "print('x := ' + '('*$n + \"'a'\" + ')'*$n + '\nr:\n  @echo hi\n')" > deep2.just; /target/debug/just -f deep2.just r 2>&1 | head -1 | cut -c1-80; done
