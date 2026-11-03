p='tmp/p1/src/bin/p4.rs'
s=open(p).read()
import re
# neutralise noisy float-dependent comparisons: compare value trees loosely by ignoring numbers
s=s.replace('fn conv(v:&serde_json::Value)->String{ serde_json::to_string(v).unwrap() }','''fn conv(v:&serde_json::Value)->String{ serde_json::to_string(v).unwrap() }
fn strip(v:&serde_json::Value)->serde_json::Value{ use serde_json::Value as V; match v { V::Number(_)=>V::Null, V::Array(a)=>V::Array(a.iter().map(strip).collect()), V::Object(o)=>V::Object(o.iter().map(|(k,v)|(k.clone(),strip(v))).collect()), x=>x.clone() } }''')
s=s.replace('if &sa!=b {','if strip(&sa)!=strip(b) {')
s=s.replace('if &re!=b {','if strip(&re)!=strip(b) {')
s=s.replace('if &o!=b {','if strip(&o)!=strip(b) {')
s=s.replace('println!("=== {k}: {}", v.len());\n        for s in v.iter().take(14)','println!("=== {k}: {}", v.len());\n        for s in v.iter().rev().take(if k=="acc_nul"||k=="rej_invalidnumber"||k=="acc_trailing"||k=="borrowed_diff"||k=="serde_owned_diff"{4}else{25})')
open(p,'w').write(s)
