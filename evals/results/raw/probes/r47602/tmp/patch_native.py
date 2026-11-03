p='tmp/cn/src/impls/mod.rs'
s=open(p).read()
s=s.replace('''    not(target_arch = "aarch64"),
''','')
open(p,'w').write(s)
p='tmp/cn/src/lib.rs'
s=open(p).read()
s=s.replace('''        let input: SillyWrapper = SillyWrapper::from(input);
        impls::neon::parse_str(input, data, buffer, idx)''','''        let input: SillyWrapper = SillyWrapper::from(input);
        unsafe { impls::native::parse_str(input, data, buffer, idx) }''')
s=s.replace('''unsafe { Self::_find_structural_bits::<impls::neon::SimdInput>(input, structural_indexes) }''','''unsafe { Self::_find_structural_bits::<impls::native::SimdInput>(input, structural_indexes) }''')
open(p,'w').write(s)
