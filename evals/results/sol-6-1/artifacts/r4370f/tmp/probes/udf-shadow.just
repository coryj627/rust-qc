set unstable
x := 'global'
y := x
get_y(x) := y
main:
  @echo '{{get_y("argument")}}'
