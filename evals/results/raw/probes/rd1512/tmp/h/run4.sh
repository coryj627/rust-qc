cd /work/tmp/h && cargo build --release --offline --bin p4 --bin pt 2>&1 | grep -E "^error" -A9 | head -30
timeout 200 /target/release/p4 2>/dev/null
/target/release/pt
python3 - <<'PY'
import unicodedata as u
got=set(int(x) for x in open("/work/tmp/punct.txt"))
want=set(c for c in range(0x110000) if u.category(chr(c))[0] in "PS")
a=sorted(want-got); b=sorted(got-want)
print(u.unidata_version,"missing",len(a),[hex(x) for x in a[:30]]); print("extra",len(b),[hex(x) for x in b[:30]])
print("extra-cats",sorted(set(u.category(chr(x)) for x in b)))
PY
# patched copy to prove root causes
cd /work/tmp && mkdir -p pc && cp -r /work/crate/pulldown-cmark /work/crate/pulldown-cmark-escape pc/ 
python3 - <<'PY'
p='/work/tmp/pc/pulldown-cmark/src/parse.rs'
s=open(p).read()
s=s.replace("""            self.lower_bounds[InlineStack::TILDES]
        }""","""            0
        }""")
s=s.replace("""                self.lower_bounds[InlineStack::UNDERSCORE_NOT_BOTH] = new_bound;
""","""                self.lower_bounds[InlineStack::UNDERSCORE_NOT_BOTH] = new_bound;
                self.lower_bounds[InlineStack::UNDERSCORE_BASE + count % 3] = new_bound;
""")
open(p,'w').write(s)
p='/work/tmp/pc/pulldown-cmark/Cargo.toml'
s=open(p).read()
open(p,'w').write(s)
PY
mkdir -p h2/src/bin && cat > h2/Cargo.toml <<'T'
[package]
name = "h2"
version = "0.0.0"
edition = "2021"
[dependencies]
pulldown-cmark = { path = "../pc/pulldown-cmark" }
[workspace]
T
cp /work/crate/Cargo.lock h2/ ; cat > h2/src/bin/x.rs <<'T'
use pulldown_cmark::*; use std::time::Instant;
fn main(){ for (s,o) in [("~~a~ b~~",Options::ENABLE_STRIKETHROUGH),("~a^ b~",Options::ENABLE_SUBSCRIPT|Options::ENABLE_SUPERSCRIPT)] { let mut h=String::new(); html::push_html(&mut h,Parser::new_ext(s,o)); println!("{:?} -> {:?}",s,h);} 
 for n in [40000,160000,640000] { let s="*a_ ".repeat(n); let st=Instant::now(); let c=Parser::new(&s).count(); println!("patched *a_ n={} ev={} t={:.3}",n,c,st.elapsed().as_secs_f64()); } }
T
cd h2 && cargo build --release --offline --bin x --target-dir /target/h2 2>&1 | grep -E "^error" -A7 | head -20; timeout 120 /target/h2/release/x
