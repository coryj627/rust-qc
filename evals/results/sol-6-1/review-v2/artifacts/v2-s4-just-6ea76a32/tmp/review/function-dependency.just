set unstable
f() := is_dependency()
@default: dep
  echo {{is_dependency()}}:{{f()}}
@dep:
  echo {{is_dependency()}}:{{f()}}
