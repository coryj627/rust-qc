set lazy
x := error('unused dependency evaluated')
main: dependency
  @echo success
dependency:
  @echo '{{x}}'
