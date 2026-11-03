import unicodedata,html.entities
p=set(int(l,16) for l in open('/work/probes/data/punct.txt'))
exp=set(c for c in range(0x80,0x110000) if unicodedata.category(chr(c))[0] in 'PS')
print('unidata',unicodedata.unidata_version)
a=sorted(p-exp); b=sorted(exp-p)
print('treated as punct but not P/S',len(a),[hex(x) for x in a[:30]],[unicodedata.category(chr(x)) for x in a[:10]])
print('P/S but not treated',len(b),[hex(x) for x in b[:40]])
bad=0
got={}
for l in open('/work/probes/data/ents_out.txt'):
    parts=l.rstrip('\n').split(' ')
    got[parts[0]]=parts[1] if len(parts)>1 else ''
for n,v in html.entities.html5.items():
    if not n.endswith(';'): continue
    nm=n[:-1]
    want='61,'+','.join('%x'%ord(c) for c in v)+',62'
    if got.get(nm)!=want:
        bad+=1
        if bad<20: print('entity mismatch',nm,got.get(nm),want)
print('entities total',len(got),'bad',bad)
