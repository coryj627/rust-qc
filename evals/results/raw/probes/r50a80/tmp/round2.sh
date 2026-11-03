cd /work/probe && cargo build --offline 2>&1 | grep -E "^error|^ +--> src|Finished" | head -30
python3 /work/tmp/mkindex.py /tmp/idx
export TMPDIR=/tmp
sh run_all.sh /target/debug/probe stash_moved_opts apply_moved_opts rebase_op_dangling rebase_exec annotated_refname tag_foreach notify_cancel notify_panic recreate_order dry_run indexer_odb from_buffer_static pack_write_leak refs_names_latin1 update_tips_latin1 remote_list_latin1 tree_walk_latin1 describe_latin1 pathspec_empty submodule_fuzz time_ord transport_err_msg remote_create_err_msg
echo === index; /target/debug/probe index_flags_overread /tmp/idx 2>&1 | cut -c1-200 | head
