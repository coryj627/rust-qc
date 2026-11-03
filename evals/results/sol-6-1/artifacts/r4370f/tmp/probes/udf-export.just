set unstable
export X := 'exported'
f() := `printf '%s' "${X-unset}"`
main:
  @echo '{{f()}}'
