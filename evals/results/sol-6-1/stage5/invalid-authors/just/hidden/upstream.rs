#[test]
fn dry_run_does_not_execute_shell_function() {
  Test::new()
    .justfile(
      "
        foo:
          echo {{ shell('exit 1') }}
      ",
    )
    .arg("--dry-run")
    .stderr("echo shell(\"exit 1\")\n")
    .success();
}

#[test]
fn dry_run_shell_function_output_is_escaped() {
  Test::new()
    .justfile(
      r#"
        foo:
          echo {{ shell('exit 1', '"') }}
      "#,
    )
    .arg("--dry-run")
    .stderr(
      r#"
        echo shell("exit 1", "\"")
      "#,
    )
    .success();
}
