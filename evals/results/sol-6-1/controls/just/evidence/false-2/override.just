set dotenv-load
set dotenv-override
value := env('CONTROL_INHERITED')

check:
    @printf 'env=<%s> shell=<%s>\n' '{{value}}' "$CONTROL_INHERITED"
