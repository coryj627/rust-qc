import hashlib,struct,sys
oid=bytes.fromhex('e69de29bb2d1d6434b8b29ae775ad8c2e48c5391')
def entry(path,flaglen,total_path_area):
    e=struct.pack('>10I',0,0,0,0,0,0,0o100644,0,0,0)+oid+struct.pack('>H',flaglen)
    e+=path+b'\0'*(total_path_area-len(path))
    return e
# flags claim a 2000-byte name; the NUL-terminated name is one byte long.
n=2000
size=(62+n+8)&~7
body=b'DIRC'+struct.pack('>II',2,1)+entry(b'a',n,size-62)
open('/work/tmp/crafted_index','wb').write(body+hashlib.sha1(body).digest())
print('ok',len(body))
