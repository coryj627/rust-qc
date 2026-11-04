from pathlib import Path
import difflib,hashlib,json
root=Path('/work/skill'); out=Path('/work/tmp/recommendation-check')
hashes={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob('*') if p.is_file()}
(out/'skill-before.json').write_text(json.dumps(hashes,sort_keys=True,indent=2)+'\n')
def replace_once(s,a,b):
 assert s.count(a)==1, (a,s.count(a))
 return s.replace(a,b,1)
def patch(name,changes):
 result=''
 for path,new in changes.items():
  old=(root/path).read_text()
  result+=''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path,n=3))
 (out/name).write_text(result)
base=(root/'SKILL.md').read_text()
a=replace_once(base,
 'Report these hygiene results compactly: one Minor finding per category (documentation, `Debug`, SAFETY comments, lint configuration), each with a count and the few instances that matter most.',
 'Report these hygiene results compactly in a separate QA hygiene section: one Minor item per category (documentation, `Debug`, SAFETY comments, lint configuration), each with a count and the few instances that matter most. Keep hygiene item counts separate from behavioral and documented-contract defect counts.')
a=replace_once(a,
 '1. A verdict of two to four sentences, including a count of findings per severity.',
 '1. A verdict of two to four sentences, including defect counts per severity and a separate count of QA hygiene items.')
a=replace_once(a,
 '2. One section per severity, in order. Each finding is a heading that names the problem in a few words, followed by a location line and a short problem and fix. One problem per finding. Within a severity, put what users hit first, then soundness, then maintainability.',
 '2. One section per severity for defects, in order. Each finding is a heading that names the problem in a few words, followed by a location line and a short problem and fix. One problem per finding. Within a severity, put what users hit first, then soundness, then maintainability. Follow with grouped QA hygiene items, retaining their practical impact and check results. A convention omission alone belongs there; a demonstrated violation of a documented contract belongs with defects. Cross-reference overlapping evidence rather than counting it twice.')
ref=(root/'references/audit-checklist.md').read_text()
ref=replace_once(ref,
 '## 2. Heuristic scan',
 'In the report, keep grouped QA hygiene separate from behavioral and documented-contract defects and their totals. Preserve failing commands and useful maintenance advice. A diagnostic is evidence of what the tool checked, not by itself evidence of incorrect behavior.\n\n## 2. Heuristic scan')
patch('01-hygiene.patch',{'SKILL.md':a,'references/audit-checklist.md':ref})
b=replace_once(base,
 '## Verification gate',
 'For each behavioral or documented-contract defect, give the expected behavior and its basis, the smallest useful reproducer or retained probe source, the exact command, and the observed result. State whether the evidence was executed, established by source inspection, or remains unverified. Separate the observed failure from its inferred cause; label a cause not traced in source as a hypothesis. An unknown cause does not invalidate a demonstrated failure. Keep unverified concerns outside confirmed defect totals.\n\n## Verification gate')
b=replace_once(b,
 '## Reference files',
 'Create and modify summaries also give the exact verification commands and actual outcomes, including the working directory, relevant environment, toolchain, package, target, and feature selection. Distinguish pass, fail, skipped, and blocked checks; identify pre-existing diagnostics and limits. Test counts alone are not a reproducible verification summary.\n\n## Reference files')
patch('02-evidence.patch',{'SKILL.md':b})
print('Generated two proposed diffs; archived skill unchanged.')
