set unstable
export x := 'ok'
f() := `printf '%s' "$x"`
@show:
  echo {{f()}}
