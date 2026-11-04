set export
own := 'child'
main:
  @printf '%s|%s\n' "${secret-unset}" "$own"
