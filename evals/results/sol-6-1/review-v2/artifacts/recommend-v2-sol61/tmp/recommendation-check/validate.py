from pathlib import Path
import json,re,hashlib,difflib,subprocess,shutil
b=Path('/work/v2');hist=Path('/work/original-sol');skill=Path('/work/skill');check=Path('/work/tmp/recommendation-check');doc=Path('/work/output/recommended-skill-fixes.md')
s=doc.read_text();body=re.sub(r'^```.*?^```\s*$', '', s, flags=re.M|re.S)
links=re.findall(r'\[[^\]]+\]\(([^)]+)\)',body)
resolved=[]
for link in links:
 assert not link.startswith('/work'),link
 link=link.split('#',1)[0]
 if link.startswith('../../'):p=Path('/work/claude')/link[6:]
 elif link.startswith('../'):p=hist/link[3:]
 else:p=b/link
 assert p.is_file() or p.is_dir(),(link,str(p))
 resolved.append(link)
print('LINKS:',len(links),'occurrences;',len(set(resolved)),'unique; all exist in publication mapping')
patches=re.findall(r'^````diff\n(.*?)^````$',s,flags=re.S|re.M)
assert len(patches)==2
scratch=check/'archive-context';scratch.mkdir(exist_ok=True)
for n in ['SKILL.md','references/audit-checklist.md']:
 p=scratch/n;p.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(skill/n,p)
for i,p in enumerate(patches,1):
 path=check/f'extracted-{i}.patch';path.write_text(p)
 r=subprocess.run(['git','apply','--check',str(path)],cwd=scratch,text=True,capture_output=True)
 assert r.returncode==0,(i,r.stdout,r.stderr)
 print('PATCH',i,'git apply --check: PASS')
# Check each hunk at its specified original lines, and build the combined proposal in memory.
def parse_patch(p):
 files={};current=None;hunks=[];h=None
 for l in p.splitlines(True):
  if l.startswith('--- a/'):
   current=l[6:].rstrip('\n');files[current]=[]
  elif l.startswith('+++ '):continue
  elif l.startswith('@@ '):
   m=re.match(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@',l);assert m,l
   h={'old_start':int(m[1])-1,'old_count':int(m[2] or 1),'new_count':int(m[4] or 1),'lines':[]};files[current].append(h)
  else:
   assert h is not None and l[:1] in [' ','+','-'],repr(l)
   h['lines'].append(l)
 return files
changes={}
for p in patches:
 for name,hunks in parse_patch(p).items():
  old=(skill/name).read_text().splitlines(True)
  for h in hunks:
   prior=[l[1:] for l in h['lines'] if l[0] in ' -'];after=[l[1:] for l in h['lines'] if l[0] in ' +']
   assert len(prior)==h['old_count'] and len(after)==h['new_count']
   assert old[h['old_start']:h['old_start']+len(prior)]==prior,(name,h['old_start'])
   changes.setdefault(name,[]).append((h['old_start'],len(prior),after))
combined=''
for name,edits in changes.items():
 old=(skill/name).read_text().splitlines(True);new=old[:]
 edits.sort(reverse=True)
 for start,count,after in edits:new[start:start+count]=after
 combined+=''.join(difflib.unified_diff(old,new,fromfile='a/'+name,tofile='b/'+name))
cp=check/'combined.patch';cp.write_text(combined)
r=subprocess.run(['git','apply','--check',str(cp)],cwd=scratch,text=True,capture_output=True)
assert r.returncode==0,(r.stdout,r.stderr)
print('COMBINED PATCH CONTEXT: PASS (no patch applied)')
quotes=[
 ('grading-blinding-limitations.json','Residual report totals can cue that additional passages were inserted.'),
 ('artifacts/control-simd-json/prepared/evidence/decoy-1.log','empty Vec=[]; empty array=[]; nested=[[],[1]]'),
 ('reports/v2-s3-moka-283a1dc7.md','9 findings: 2 Blocker, 4 Major, and 3 Minor'),
 ('reports/v2-s3-moka-283a1dc7.md','several already describe errors in prose'),
 ('reports/v2-s3-moka-a978f9f8.md','Twenty additional missing-`# Errors` advisories largely concern APIs already describing errors in prose.'),
 ('reports/v2-s3-moka-badbed44.md','five correctness defects and one documentation defect'),
 ('reports/v2-s3-moka-c0a8fef6.md','seven correctness issues and one documentation issue'),
 ('reports/verify-s3-moka.md','Several APIs already explain error behavior in prose; the asserted omission concerns documentation structure.'),
 ('artifacts/verify-s3-moka/tmp/doc-public.log','error: unresolved link to `try_append_value_async.rs`'),
 ('artifacts/verify-s3-moka/tmp/independent.log','compute values: 1,2,2'),
 ('reports/v2-s3-just-fc32aea1.md','a Display implementation returned an error unexpectedly'),
 ('reports/recheck-s3-just.md','identifies the invalid-format builtin panics, but does not identify'),
]
for path,q in quotes:assert q in (b/path).read_text(),(path,q)
grade=json.loads((hist/'quality_scores.json').read_text())
for rid in ['s5-e0ce2865','s5-a536c375']:
 row=next(x for x in grade if x['run']==rid)
 assert row['final_quality']==11.75
 assert len(row['grades'])==2
 for g in row['grades']:
  d=next(c for c in g['criteria'] if c['id']=='D2');assert d['score']==0.25
for r in ['B','C']:
 for config in ['default','no-default']:
  assert '23 passed; 0 failed' in (hist/f'stage5/assessments/decom/{r}/hidden/hidden-{config}.log').read_text()
print('QUOTATIONS AND HISTORICAL D2/HIDDEN LOG CROSS-CHECK: PASS')
before=json.loads((check/'skill-before.json').read_text())
after={str(p.relative_to(skill)):hashlib.sha256(p.read_bytes()).hexdigest() for p in skill.rglob('*') if p.is_file()}
assert before==after
print('ARCHIVED SKILL: all',len(after),'file hashes unchanged')
assert '__DIFF' not in s and '__METRICS' not in s and '__REPLICATES' not in s
assert len(re.findall(r'^````(?:diff)?$', s, flags=re.M)) == 4
result={'links':len(links),'unique_links':len(set(resolved)),'patches_checked':2,'combined_context_checked':True,'all_skill_hashes_unchanged':True,'rust_executed':False,'document_sha256':hashlib.sha256(doc.read_bytes()).hexdigest()}
(check/'validation.json').write_text(json.dumps(result,indent=2)+'\n')
