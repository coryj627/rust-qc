set unstable
x := uuid()
f() := x
@show:
  echo {{if x == f() { 'same' } else { 'different' }}}
