import hashlib,struct,sys
# One entry: on-disk flags claim a 1000-byte name, but the name ends after one byte.
plen=1000
path=b"a"+b"\0"*(plen-1)
entry=struct.pack(">IIIIIIIIII",0,0,0,0,0,0,0o100644,0,0,0)+b"\x11"*20+struct.pack(">H",plen)+path
sz=(62+plen+8)&~7
entry=entry+b"\0"*(sz-len(entry))
body=b"DIRC"+struct.pack(">II",2,1)+entry
open(sys.argv[1],"wb").write(body+hashlib.sha1(body).digest())
