import struct,zlib,zipfile,io
from pathlib import Path
root=Path('/work/tmp/fixtures')
def archive(comment=b'',extra=b'',name=b'a'):
 data=b'hello';crc=zlib.crc32(data)
 local=struct.pack('<4s5H3I2H',b'PK\x03\x04',20,0,0,0,33,crc,len(data),len(data),len(name),0)+name+data
 central=struct.pack('<4s6H3I5H2I',b'PK\x01\x02',0x314,20,0,0,0,33,crc,len(data),len(data),len(name),len(extra),len(comment),0,0,0o100644<<16,0)+name+extra+comment
 end=struct.pack('<4s4H2IH',b'PK\x05\x06',0,0,1,1,len(central),len(local),0)
 return local+central+end
def uni(kind,orig,utf):
 body=b'\x01'+struct.pack('<I',zlib.crc32(orig))+utf
 return struct.pack('<HH',kind,len(body))+body
for n,c,e in [('cp_comment',b'caf\x82',b''),('unicode_comment',b'caf\x82',uni(0x6375,b'caf\x82','café'.encode())),('unicode_ascii',b'cafe',uni(0x6375,b'cafe',b'cafe')),('stale_path',b'',uni(0x7075,b'old',b'old')),('long_comment',b'\x82'*40000,b'')]:
 d=archive(c,e);(root/(n+'.zip')).write_bytes(d)
 with zipfile.ZipFile(io.BytesIO(d)) as z: assert z.read('a')==b'hello'
 print(n,len(d),'python control passed')
