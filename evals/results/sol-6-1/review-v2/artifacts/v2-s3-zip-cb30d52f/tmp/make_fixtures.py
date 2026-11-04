import io, pathlib, struct, zipfile, zlib
root = pathlib.Path('/work/tmp/fixtures')
def extra(kind, content):
    return struct.pack('<HH', kind, len(content)) + content
for raw, stem in [(b'\x82', 'cp437-comment'), (b'plain', 'ascii-comment')]:
    info = zipfile.ZipInfo('a')
    info.comment = raw
    info.extra = extra(0x6375, b'\x01' + struct.pack('<I', zlib.crc32(raw)) + 'comment-é'.encode())
    with zipfile.ZipFile(root / (stem + '.zip'), 'w') as z:
        z.writestr(info, b'hello')
# A high but valid UTF-8 comment stored as CP437 grows past the 16-bit byte limit on rewriting.
info = zipfile.ZipInfo('a')
info.comment = b'\x82' * 40000
with zipfile.ZipFile(root / 'cp437-long-comment.zip', 'w') as z:
    z.writestr(info, b'hello')
# A valid ZIP64 file with five-byte payload and a local header above the 32-bit offset limit.
# The sparse prefix is supporting data, like a self-extracting executable prefix.
local_offset = 0x100000000
name = b'a'
crc = zlib.crc32(b'hello')
with (root / 'sparse-offset.zip').open('wb') as f:
    f.seek(local_offset)
    f.write(struct.pack('<IHHHHHIIIHH', 0x04034b50, 45, 0, 0, 0, 33, crc, 5, 5, 1, 0) + name + b'hello')
    cd_offset = f.tell()
    field = extra(1, struct.pack('<Q', local_offset))
    f.write(struct.pack('<IHHHHHHIIIHHHHHII', 0x02014b50, 45, 45, 0, 0, 0, 33, crc, 5, 5, 1, len(field), 0, 0, 0, 0, 0xffffffff) + name + field)
    cd_size = f.tell() - cd_offset
    zip64_offset = f.tell()
    f.write(struct.pack('<IQHHIIQQQQ', 0x06064b50, 44, 45, 45, 0, 0, 1, 1, cd_size, cd_offset))
    f.write(struct.pack('<IIQI', 0x07064b50, 0, zip64_offset, 1))
    f.write(struct.pack('<IHHHHIIH', 0x06054b50, 0, 0, 1, 1, cd_size, 0xffffffff, 0))
for stem in ['cp437-comment', 'ascii-comment', 'cp437-long-comment', 'sparse-offset']:
    with zipfile.ZipFile(root / (stem+'.zip')) as z:
        print(stem, 'Python read', z.read('a'), 'header_offset', z.getinfo('a').header_offset)

with zipfile.ZipFile(root / 'cp437-short-comment.zip', 'w') as z:
    info = zipfile.ZipInfo('a')
    info.comment = b'\x82'
    z.writestr(info, b'hello')
