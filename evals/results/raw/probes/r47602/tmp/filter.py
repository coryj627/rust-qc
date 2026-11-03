import re,sys
p='tmp/p1/src/bin/p4.rs'
s=open(p).read()
# only keep the accept/reject + invalid categories, and classify
s=s.replace('(Ok(a),Err(e)) => note("simd_accepts_sj_rejects", format!("{show:?} simd={a:?} sj={e}")),',
 '''(Ok(a),Err(e)) => { let t = show.clone(); let nul = t.contains('\\0'); let lone = t.contains("\\\\ud") || t.contains("\\\\uD") ; let es=e.to_string(); let cls = if nul {"acc_nul"} else if lone && (es.contains("hex")||es.contains("surrogate")) {"acc_surrogate"} else if es.contains("trailing char")||es.contains("expected `,`") {"acc_trailing"} else {"acc_other"}; note(cls, format!("{show:?} simd={a:?} sj={e}")) },''')
s=s.replace('(Err(e),Ok(b)) => note("sj_accepts_simd_rejects", format!("{show:?} {e} sj={b}")),',
 '''(Err(e),Ok(b)) => { let es=format!("{e}"); let big = es.starts_with("InvalidNumber"); note(if big {"rej_invalidnumber"} else {"rej_other"}, format!("{show:?} {e} sj={b}")) },''')
for k in ['encode_diff','encode_pp_diff','pretty_diff','tape_encode_diff','to_string_diff','serde_sjv_diff','value_diff']:
    s=s.replace('note("%s",'%k,'if !show.contains(|c:char|c==\'.\'||c==\'e\'||c==\'E\'||c==\'-\') && false {} else if false {} else if true { let _=1; } else {note("%s",'%k) if False else s
open(p,'w').write(s)
