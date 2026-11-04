set unstable
x := 'original'
get_x() := x
main:
  @printf '%s|%s\n' '{{x}}' '{{get_x()}}'
