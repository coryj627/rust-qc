cd /work/tmp/p3; J=/work/tmp/bin/just
t() { echo "=== $*"; "$@" 2>&1 | head -${H:-12}; }
t $J -f e1
t $J -f al1 --list
t $J -f al1 --list --alias-style separate
t $J -f al1 x
t $J -f pm --summary
t $J -f pm --list
t $J -f pm --choose --chooser 'cat >&2; echo top'
t $J -f dp
t $J -f ev --evaluate --evaluate-format shell
$J -f ev --evaluate --evaluate-format shell > ev.sh; echo sh:; sh -c '. ./ev.sh; printf "%s\n" "$x"'; bash -c '. ./ev.sh; printf "%s\n" "$x"'
t $J -f bl
t $J -f bl2
t $J -f lz --yes
t $J -f lz2 other
for sub in --list --summary --variables --dump --man --changelog --groups --evaluate '--completions bash' '--show recipe1' ; do
  $J -f big $sub 2>err.txt | true; echo "$sub rc=${PIPESTATUS:-?}"; head -3 err.txt
done
echo DIAMOND; timeout 20 $J -f diamond2 a0 | tail -1; echo rc=$?; timeout 20 $J -f diamond a0 2>&1 | tail -1; echo rc=$?
for a in "--show ''" "--usage ''" "--list ''" "--evaluate ''" "--show ::" "--list ::" "--evaluate ::" "--usage a::" ; do eval "t $J -f bl $a"; done
