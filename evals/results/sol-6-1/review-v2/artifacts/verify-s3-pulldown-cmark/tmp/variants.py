import subprocess,json
exe='/target/debug/pulldown-cmark'
cases=[
 ('tilde-tab-close',[], '~~~\nx\n~~~\t\nafter\n'),
 ('blockquote-crlf-code',[], '> `a\r\n> b`\r\n'),
 ('cr-fence',[], '```rust\rx\r```'),
 ('cr-html',[], '<pre>\rx\r</pre>\r\rafter\r'),
 ('pre-uppercase',[], '<pre>\nx\n</PRE>\n*after*\n'),
 ('heading-pretab',[], '# heading\t###\n'),
 ('heading-posttab',[], '# heading ###\t\n'),
 ('hardbreak-spacetab',[], 'foo \t\nbar'),
 ('hardbreak-tabspace',[], 'foo\t \nbar'),
 ('subscript-link',['-B'], '~~[a](b)~~'),
 ('subscript-code',['-B'], '~~`a`~~'),
 ('wiki-image-range',['-W','-e'], '![[foo.png]]'),
 ('wiki-piped-range',['-W','-e'], '[[Wiki|text]]'),
]
for name,args,s in cases:
 p=subprocess.run([exe,*args],input=s.encode(),capture_output=True)
 print(json.dumps({'case':name,'exit':p.returncode,'output':p.stdout.decode(),'stderr':p.stderr.decode()},ensure_ascii=False))
