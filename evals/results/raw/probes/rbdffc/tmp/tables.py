import re,zlib
src=open('/work/crate/src/cp437.rs').read()
pairs=re.findall(r'0x([0-9a-f]{2}) => 0x([0-9a-f]{4})',src)
bad=[(a,b,bytes([int(a,16)]).decode('cp437')) for a,b in pairs if bytes([int(a,16)]).decode('cp437')!=chr(int(b,16))]
print('cp437 pairs',len(pairs),'mismatches',bad)
src=open('/work/crate/src/zipcrypto.rs').read()
tab=[int(x,16) for x in re.findall(r'0x([0-9a-f]{8})',src[src.index('CRCTABLE'):])]
ref=[]
for i in range(256):
    c=i
    for _ in range(8): c=(c>>1)^0xedb88320 if c&1 else c>>1
    ref.append(c)
print('crc table', len(tab), tab==ref)
