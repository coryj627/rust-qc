from pathlib import Path
root = Path('/work/prepared/evidence')
fixtures = {
 'decoy-1/justfile': "@default: (emit 'one') (emit 'two') (emit 'one')\n  printf 'done\\n'\n\n@emit item:\n  printf '%s\\n' '{{item}}'\n",
 'decoy-2/justfile': "value := `printf '  payload \\t\\n'`\nno_newline := `printf 'payload \\t'`\nmultiline := `printf 'payload\\n\\n'`\n",
 'false-1/justfile': "[arg('name', long='name')]\n@show name='fallback':\n  printf '<%s>\\n' '{{name}}'\n",
 'false-1/required.justfile': "[arg('name', long='name')]\n@show name:\n  printf '<%s>\\n' '{{name}}'\n",
 'false-2/justfile': "truthy := if 'a' == 'a' { 'chosen-then' } else { error('UNSELECTED_ELSE') }\nfalsy := if 'a' == 'b' { error('UNSELECTED_THEN') } else { 'chosen-else' }\n",
 'false-2/selected.justfile': "value := if 'a' == 'a' { error('SELECTED_ERROR') } else { 'other' }\n",
 'false-3/justfile': "value := '[' + env('CONTROL_EMPTY', 'fallback') + ']'\n",
 'false-4/justfile': "@gather +items:\n  printf '<%s>\\n' '{{items}}'\n",
 'false-4/star.justfile': "@gather *items:\n  printf '<%s>\\n' '{{items}}'\n",
 'false-5/justfile': "[arg('mode', pattern='dev|prod')]\n@build mode: dependency\n  printf 'body:%s\\n' '{{mode}}'\n\n@dependency:\n  printf 'dependency-ran\\n'\n",
}
for name, contents in fixtures.items():
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents)
print(f'Preserved {len(fixtures)} justfile fixtures for seven controls.')
