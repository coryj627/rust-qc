exec(open('/work/tmp/review/probe.py').read().split("case('format-private'")[0])
case('shared-import',{'justfile':"mod first\nmod second\n",'first.just':"import 'common.just'\n",'second.just':"import 'common.just'\n",'common.just':"x := 'default'\ntask:\n  @echo {{module_path()}} {{x}}\n"},[['first::task','second::task'],['--set','first::x','override','first::task','second::task']])
case('shared-module-context',{'justfile':"mod first 'shared.just'\nmod second 'shared.just'\n",'shared.just':"x := 'default'\ntask:\n  @echo {{module_path()}} {{x}}\n"},[['first::task','second::task'],['--set','first::x','override','first::task'],['--set','second::x','override','first::task','second::task']])
case('alias-list',{'justfile':"alias shortcut := helper::task\nmod helper\n",'helper.just':"task:\n  @echo helper\n"},[['--list'],['--list','--alias-style','separate'],['shortcut']])
case('alias-wrong-target',{'justfile':"alias shortcut := helper::task\nmod helper\ntask:\n  @echo root\n",'helper.just':"task:\n  @echo helper\n"},[['--list'],['--show','shortcut'],['shortcut']])
case('ceiling/parent',{'justfile':'task:\n  @echo parent\n','child/placeholder':''},[])
d=root/'ceiling/parent/child'
for ceiling in ['.',str(d),'../child',str(d/'..'/'child')]:
    p=subprocess.run([bin,'--ceiling',ceiling],cwd=d,text=True,capture_output=True)
    print('CEILING',repr(ceiling),'exit',p.returncode,'stdout',repr(p.stdout),'stderr',repr(p.stderr),flush=True)
case('timestamp',{'justfile':'task:\n  @echo done\n'},[['--timestamp','--timestamp-format','%Q']])
case('group',{'justfile':"[group('foo')]\ntask:\n  @echo done\n"},[['--choose','--chooser','head -n1','--group','foo']])
