p='probes/meta/src/main.rs'
s=open(p).read()
s=s.replace('if shown[i]<6 && (i==0||i==2||i==3) {','if shown[i]<40 && (i==2||i==3) && !md.contains(\'\\t\') && !md.contains("---\\n") {')
s=s.replace('println!("[{}/{}{}] {:?}\\n   base: {:?}\\n   var : {:?}",f,name,if pan{" PANIC"}else{""},md,b,','println!("[{}/{}{}] {:?}\\n   base: {:?}\\n   var : {:?}",f.split(\'/\').last().unwrap(),name,if pan{" PANIC"}else{""},md,b,')
open(p,'w').write(s)
