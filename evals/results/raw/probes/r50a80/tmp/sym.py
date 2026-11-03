import re,subprocess,sys
bin=sys.argv[1]
out=open(sys.argv[2],errors='replace').read()
addrs=set(re.findall(r'\(probe\+(0x[0-9a-f]+)\)',out))
m={}
if addrs:
    a=sorted(addrs)
    r=subprocess.run(['addr2line','-f','-C','-e',bin]+a,capture_output=True,text=True).stdout.split('\n')
    for i,x in enumerate(a):
        m[x]=r[2*i][:70]+' '+r[2*i+1].split('/')[-1][:30]
for l in out.split('\n'):
    k=re.search(r'\(probe\+(0x[0-9a-f]+)\)',l)
    if k: l=re.sub(r'\s+#(\d+).*',lambda mm:'  #'+mm.group(1)+' '+m.get(k.group(1),'?'),l)
    if re.match(r'\s*(WARNING|Write of|Read of|Previous|  #|SUMMARY|Location)',l): print(l[:130])
