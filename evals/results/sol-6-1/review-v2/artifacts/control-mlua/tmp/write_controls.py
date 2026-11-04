import json
from pathlib import Path
out = Path('/work/prepared')
command = 'cd /work/tmp/mlua-controls && cargo test --offline --locked --features lua54,vendored,serde,macros --test negative_controls -- --nocapture --test-threads=1'

def ref(test, evidence, refs):
    return {'method':'ran', 'evidence':f'Public-API regression {test} passed on the unmodified mlua 0.11.4 source copied to /work/tmp/mlua-controls. {evidence} Probe source: /work/prepared/evidence/negative_controls.rs. Observed results: /work/prepared/evidence/probe.log.', 'commands':[command], 'source_refs':refs}

controls = {
 'project':'mlua',
 'decoys':[
  {
   'id':'decoy-1', 'kind':'wrong-behavior',
   'summary':'Table::raw_insert rejects the append position idx == raw_len() + 1 as out of bounds. Consequently raw_insert(1, value) cannot populate an empty table, and inserting immediately after the last element of a nonempty sequence returns an error instead of appending.',
   'location':['src/table.rs:290'],
   'match_rule':'The report must identify rejection of the valid append position len + 1 by Table::raw_insert, including position 1 for an empty sequence, and the resulting failure to append. Claims about invalid keys in raw_set, sparse-table length ambiguity, or metamethod invocation do not match.',
   'refutation':ref('decoy_1_raw_insert_append_boundary', 'The bounds check rejects only idx < 1 or idx > size + 1; equality with size + 1 is explicitly accepted. At the append position, the shifting range idx..=size is empty and lua_rawseti stores the new value at idx. The probe successfully inserted at 1 into an empty table and at 2 into the resulting one-element table, then observed ["first", "second"]. Indices 0 and len + 2 returned Err without changing the length.', ['src/table.rs:288','src/table.rs:290','src/table.rs:303','src/table.rs:308'])
  },
  {
   'id':'decoy-2', 'kind':'wrong-behavior',
   'summary':'On Lua 5.4, Function::set_environment overwrites the shared _ENV upvalue of sibling closures. Setting a new environment for one closure therefore also redirects global lookups in another closure returned by the same chunk, even though only the first function was passed to set_environment.',
   'location':['src/function.rs:323','src/function.rs:333'],
   'match_rule':'The report must allege that Function::set_environment on Lua 5.4 mutates the existing shared _ENV cell and thereby changes global lookups for an untargeted sibling closure. Mutating entries of an explicitly shared environment table is a different behavior and does not match.',
   'refutation':ref('decoy_2_set_environment_isolates_sibling_closures', 'The Lua 5.4 branch creates a separate anonymous function with the supplied environment and uses lua_upvaluejoin to redirect only the target closure\'s _ENV slot to that function\'s upvalue. It does not overwrite the old shared cell. Two sibling closures initially had the same environment table. After setting the first environment, their results were 20 and 10; setting the sibling environment then yielded 20 and 30, while the original global marker stayed 10.', ['src/function.rs:319','src/function.rs:323','src/function.rs:326','src/function.rs:329','src/function.rs:333'])
  }
 ],
 'false_findings':[
  {
   'id':'false-1',
   'title':'Replacing a nil RegistryKey reports success but still reads back nil',
   'location':'src/state.rs:1915',
   'body':'A RegistryKey created with Value::Nil holds LUA_REFNIL. When Lua::replace_registry_value is later called with a non-nil value, the replacement is written under that unchanged sentinel rather than obtaining a normal registry slot. Since registry_value special-cases LUA_REFNIL to return nil, replacing such a key with "alpha" returns Ok but registry_value::<Value>(&key) still yields Nil. This breaks initialization patterns that create an empty registry handle and populate it later.',
   'refutation':ref('false_1_replace_nil_registry_key_allocates_independent_slot', 'The (value, LUA_REFNIL) match arm calls create_registry_value(value) and updates the existing key with new_key.take(). The sentinel is not retained for the non-nil value. The probe read back "alpha" after replacing an initially nil key, then successfully exercised alpha -> nil -> beta. A second nil key remained nil and a separate live string key remained unchanged.', ['src/state.rs:1825','src/state.rs:1860','src/state.rs:1910','src/state.rs:1915','src/state.rs:1917','src/state.rs:1918'])
  },
  {
   'id':'false-2',
   'title':'A failed AnyUserData::take leaves borrowed userdata destructed',
   'location':'src/userdata.rs:696',
   'body':'AnyUserData::take installs the destructed state before it has finished checking whether the payload can be exclusively accessed. Calling take::<T>() while a UserDataRef<T> is alive returns UserDataBorrowMutError but leaves the original userdata invalidated. Once the borrow is dropped, a later borrow or take fails instead of accessing the unchanged payload, so a recoverable borrow conflict permanently consumes the object.',
   'refutation':ref('false_2_failed_take_preserves_borrowed_userdata', 'has_exclusive_access() is checked before calling take_userdata; the failure branch only returns UserDataBorrowMutError. The probe held a shared borrow of Payload(42), observed that precise error from take, and successfully made another shared borrow while the first remained alive. After dropping it, a mutable borrow updated the payload to 43 and a subsequent take returned Payload(43). Only the successful take made later borrowing fail.', ['src/userdata.rs:691','src/userdata.rs:696','src/userdata.rs:697','src/userdata.rs:699'])
  },
  {
   'id':'false-3',
   'title':'The integer stack fast path wraps out-of-range Lua integers into u8',
   'location':'src/conversion.rs:864',
   'body':'The optimized FromLua::from_stack implementation for narrow integer types accepts a successful lua_tointegerx conversion as sufficient and truncates the resulting Lua integer to the Rust target type. Thus evaluating "return 256" as u8 succeeds with 0 and evaluating "return -1" succeeds with 255, rather than returning FromLuaConversionError. Rust callers receiving numeric Lua results can silently accept values outside the requested type\'s range.',
   'refutation':ref('false_3_u8_stack_conversion_rejects_integer_overflow', 'After lua_tointegerx succeeds, the fast path still uses checked num_traits::cast(i) and converts None into FromLuaConversionError with message "out of range". The regular from_lua path also uses checked cast. The macro is instantiated for u8. The probe observed FromLuaConversionError for both 256 and -1 through Lua chunk evaluation and direct u8::from_lua, while 0 and 255 converted successfully.', ['src/conversion.rs:11','src/conversion.rs:831','src/conversion.rs:850','src/conversion.rs:862','src/conversion.rs:864','src/conversion.rs:879'])
  },
  {
   'id':'false-4',
   'title':'Serde rejects shared acyclic child tables as recursive',
   'location':'src/serde/de.rs:745',
   'body':'The deserializer shares a visited-table set across sibling values and retains each table pointer for the entire conversion. With the default deny_recursive_tables setting, converting local child = {answer = 42}; return {left = child, right = child} into a map fails on the second occurrence with "recursive table detected", although the input has no cycle. This prevents deserializing ordinary Lua structures that reuse one child table in multiple fields.',
   'refutation':ref('false_4_serde_accepts_shared_acyclic_child_table', 'The visited set tracks the active traversal path. deserialize_map and deserialize_seq hold a local RecursionGuard, whose Drop implementation removes the table pointer when that child conversion ends. The next sibling can therefore visit the same table again. Under default LuaSerdeExt::from_value options, the exact shared-child map converted successfully with both left.answer and right.answer equal to 42. A sequence containing the same child table twice also converted to [[42], [42]].', ['src/serde/de.rs:303','src/serde/de.rs:351','src/serde/de.rs:722','src/serde/de.rs:725','src/serde/de.rs:730','src/serde/de.rs:732','src/serde/de.rs:745'])
  },
  {
   'id':'false-5',
   'title':'Function::bind drops a trailing nil from the bound argument prefix',
   'location':'src/function.rs:207',
   'body':'Function::bind loses trailing nil values when reconstructing the saved argument prefix from closure upvalues. Binding (11, nil, 33, nil) and subsequently calling the bound function with 44 invokes the original function with (11, nil, 33, 44), so select("#", ...) sees four arguments and the caller-supplied value shifts into the fourth slot. A bound nil placeholder therefore changes argument positions instead of remaining part of the prefix.',
   'refutation':ref('false_5_bind_preserves_trailing_nil_arguments', 'bind records args.len() as an explicit upvalue and stores every bound Value, including nil. The wrapper pushes exactly nbinds upvalues, rotates them ahead of call arguments, and returns nargs + nbinds, preserving nil positions independently of table length or non-nil contents. The probe bound (11, nil, 33, nil), called with 44, and observed (count=5, 11, nil, 33, nil, 44). Calling the same bound function without new arguments preserved count=4 and the final nil.', ['src/function.rs:203','src/function.rs:204','src/function.rs:207','src/function.rs:211','src/function.rs:214','src/function.rs:220','src/function.rs:221','src/function.rs:235','src/function.rs:236'])
  }
 ]
}
(out/'controls.json').write_text(json.dumps(controls, indent=2)+'\n')
log = (out/'evidence/probe.log').read_text()
start = log.index('    Finished `test`')
(out/'evidence/probe.log').write_text('Command: '+command+'\nPlatform: aarch64 Linux; rustc 1.99.1 (58cf8f9c3 2026-10-13); cargo 1.99.1 (5f94df478 2026-08-27)\nSource: scratch copy of /work/crate; only tests/negative_controls.rs added\n\n'+log[start:])
# Preserve exact numbered source evidence, independently of the scratch copy.
regions = {'src/table.rs':[(284,310)],'src/function.rs':[(201,241),(294,341)],'src/state.rs':[(1822,1827),(1853,1868),(1893,1927)],'src/userdata.rs':[(685,704)],'src/conversion.rs':[(8,12),(826,880)],'src/serde/de.rs':[(78,88),(294,317),(345,364),(713,750)]}
excerpts=[]
for rel, spans in regions.items():
    lines=(Path('/work/crate')/rel).read_text().splitlines()
    for lo,hi in spans:
        excerpts.append(f'{rel}:{lo}-{hi}\n'+'\n'.join(f'{n:5}: {lines[n-1]}' for n in range(lo,hi+1)))
(out/'evidence/source-excerpts.txt').write_text('\n\n'.join(excerpts)+'\n')
(out/'evidence/README.txt').write_text('''Seven independently specified functional negative controls for mlua 0.11.4.

controls.json uses baseline-relative source paths, rooted at /work/crate.
negative_controls.rs is the preserved public-API probe source. An identical copy
was added as tests/negative_controls.rs in /work/tmp/mlua-controls; production
source and manifests were unchanged. probe.log records the actual seven-test
run, with compilation progress omitted. source-excerpts.txt preserves numbered
baseline lines. baseline-sha256.json records all original crate file hashes.
validate_controls.py checks counts, schema, source references, successful test
identities, original-tree integrity, and scratch-source identity.

Runtime scope: aarch64 Linux, stable Rust 1.99.1, Lua 5.4 with vendored, serde,
and macros features. Other Lua backends, Miri, and the full upstream test suite
were not run. The environment control specifically concerns Lua 5.4.
All probes were small, local, synchronous functional API calls.

Known-defect exclusion review: none of the controls concerns invalid raw_set
keys, table-to-slice equality, integer-set round trips, or mixed integer/float
Value equality. The table probe uses valid integer insertion positions and
collects values into a Rust Vec for comparison. The integer probe concerns
checked conversion to u8, not Value equality.
''')
