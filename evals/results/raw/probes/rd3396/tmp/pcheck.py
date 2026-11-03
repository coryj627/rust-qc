import unicodedata,sys
print("unicode",unicodedata.unidata_version)
p=set(int(x,16) for x in open('/work/tmp/punct.txt').read().split())
w=set(int(x,16) for x in open('/work/tmp/ws.txt').read().split())
exp={u for u in range(0x80,0x110000) if not (0xD800<=u<0xE000) and unicodedata.category(chr(u))[0] in 'PS'}
print("table treats as punct but not P/S:",[hex(x) for x in sorted(p-exp)][:40],len(p-exp))
print("P/S but not treated as punct:",[hex(x) for x in sorted(exp-p)][:60],len(exp-p))
ws={u for u in range(0x80,0x110000) if not (0xD800<=u<0xE000) and unicodedata.category(chr(u))=='Zs'}
print("whitespace-treated:",[hex(x) for x in sorted(w)]);print("Zs:",[hex(x) for x in sorted(ws)])
