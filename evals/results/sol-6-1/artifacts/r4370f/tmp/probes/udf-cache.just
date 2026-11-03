set unstable
x := uuid()
get_x() := x
main:
  @printf '%s|%s|%s\n' '{{x}}' '{{get_x()}}' '{{get_x()}}'
