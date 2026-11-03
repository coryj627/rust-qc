cd /work/tmp/p14; J=/work/tmp/bin/just
$J -E .env.prod show; $J -E .env.prod m::show
cp ac ac.t; $J --fmt -f ac.t 2>&1 | tail -1; cat ac.t
