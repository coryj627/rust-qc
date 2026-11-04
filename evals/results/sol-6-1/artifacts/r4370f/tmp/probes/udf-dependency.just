set unstable
dep() := is_dependency()
main: child
child:
  @printf '%s|%s\n' '{{is_dependency()}}' '{{dep()}}'
