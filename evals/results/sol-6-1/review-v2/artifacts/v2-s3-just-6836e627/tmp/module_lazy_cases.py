exec(open('/work/tmp/regressions.py').read().split("case('private-module'")[0])
case('module-lazy',"mod sub\n", ['sub::show'], {'sub.just':"set lazy\nunused := `exit 9`\n@show:\n  echo hello\n"})
case('root-lazy-module-eager',"set lazy\nmod sub\n", ['sub::show'], {'sub.just':"unused := `exit 9`\n@show:\n  echo hello\n"})
case('dotenv-path-fallback',"set dotenv-path := 'missing.env'\nset dotenv-required\n@show:\n  echo $VALUE\n", ['show'], {'.env':"VALUE=unexpected\n"})
