#!/usr/bin/env python3
import pathlib, json, os, subprocess
for letter in __import__('sys').argv[1:]:
 out=pathlib.Path('/work/results')/letter
 scratch=pathlib.Path('/work/tmp')/('assessment-'+letter)
 cwd=scratch/'verification-source'
 env=dict(os.environ,CARGO_HOME='/work/.cargo-home',CARGO_NET_OFFLINE='true',RUSTUP_AUTO_INSTALL='0',CARGO_TERM_COLOR='never',CARGO_TARGET_DIR=str(scratch/'verification-target'),CARGO_BUILD_JOBS='2')
 features='alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes'
 records=json.loads((out/'checks.json').read_text())
 for name,cmd,extra in [('strict-clippy',['cargo','clippy','--workspace','--all-targets','--features',features,'--','-D','warnings'],{}),('strict-rustdoc',['cargo','doc','--workspace','--no-deps','--document-private-items','--features',features],{'RUSTDOCFLAGS':'-D warnings'})]:
  log=out/(name+'.log'); e=dict(env,**extra)
  with log.open('w') as f:
   f.write('cwd: '+str(cwd)+'\ncommand: '+json.dumps(cmd)+'\nenv: '+json.dumps({k:v for k,v in e.items() if k.startswith(('CARGO_','RUST','MIRI'))},sort_keys=True)+'\n'); f.flush()
   r=subprocess.run(cmd,cwd=cwd,env=e,stdout=f,stderr=subprocess.STDOUT)
   f.write('\nexit_code: '+str(r.returncode)+'\n')
  records.append(dict(name=name,command=cmd,cwd=str(cwd),exit_code=r.returncode,log=str(log),observed_outcome='completed'))
  print(letter,name,r.returncode)
 (out/'checks.json').write_text(json.dumps(records,indent=2)+'\n')
