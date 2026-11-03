import zipfile,sys,glob
for f in sorted(glob.glob('/work/tmp/x_*.zip')+glob.glob('/work/tmp/cnt_*.zip')+['/work/tmp/trunc.zip','/work/tmp/fake_eocd.zip','/work/tmp/st_nodesc.zip','/work/tmp/st_crypto.zip','/work/tmp/aes_after.zip','/work/tmp/aes_before.zip','/work/tmp/ua_chinese.zip']):
    try:
        z=zipfile.ZipFile(f); n=len(z.namelist()) if len(z.namelist())>5 else z.namelist(); print(f.split('/')[-1],'py ok',n,end=' ')
        try:
            print(z.testzip() if 'crypto' not in f and 'aes' not in f else z.read(z.namelist()[0],pwd=b'pw'))
        except Exception as e: print('read err',repr(e)[:80])
    except Exception as e: print(f.split('/')[-1],'py err',repr(e)[:90])
try:
    import lzma; print('py lzma', lzma.decompress(open('/work/tmp/empty.xz','rb').read()))
except Exception as e: print('py lzma err',repr(e))
