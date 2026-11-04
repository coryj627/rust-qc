set unstable
f() := x
x := 'child'
show:
  @echo '{{f()}} {{x}}'
