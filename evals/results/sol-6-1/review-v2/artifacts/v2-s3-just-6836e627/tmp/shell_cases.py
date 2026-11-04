from pathlib import Path
import subprocess,json
root=Path('/work/tmp/review-cases/shell-values'); root.mkdir(exist_ok=True)
for label,value in [('bang','hello!world'),('special','!"$\\`'),('unicode','hello π')]:
 (root/'justfile').write_text('foo := '+"'"+value+"'\n")
 p=subprocess.run(['/target/debug/just','--evaluate','--evaluate-format','shell'],cwd=root,capture_output=True,text=True,timeout=5)
 for shell in ['sh','bash']:
  q=subprocess.run([shell,'-c',p.stdout+'printf "%s" "$foo"'],capture_output=True,text=True,timeout=5)
  print(json.dumps({'case':label,'shell':shell,'input':value,'output_from_just':p.stdout,'sourced_status':q.returncode,'sourced_value':q.stdout,'matches':q.stdout==value}))
