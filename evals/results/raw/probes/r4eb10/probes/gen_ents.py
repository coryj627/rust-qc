import html.entities
open('/work/probes/data/ents.txt','w').write('\n'.join(n[:-1] for n in html.entities.html5 if n.endswith(';'))+'\n')
