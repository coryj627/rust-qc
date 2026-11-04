set unstable
y := 'global'
x := y
f(y) := x
@show:
  echo {{f('parameter')}}
