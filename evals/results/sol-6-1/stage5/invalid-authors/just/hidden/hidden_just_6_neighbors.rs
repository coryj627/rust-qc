use super::*;

#[test]
fn dry_run_assignment_and_interpolation_have_no_side_effects() {
  let output = Test::new()
    .justfile(
      "
        value := shell('touch assignment-marker; printf value')
        foo:
          echo {{value}}
          echo {{shell('touch interpolation-marker; printf other')}}
      ",
    )
    .arg("--dry-run")
    .stderr(
      "echo shell(\"touch assignment-marker; printf value\")\necho shell(\"touch interpolation-marker; printf other\")\n",
    )
    .success();
  assert!(!output.tempdir.path().join("assignment-marker").exists());
  assert!(!output.tempdir.path().join("interpolation-marker").exists());
}

#[test]
fn dry_run_needs_no_shell_executable() {
  Test::new()
    .shell(false)
    .args(["--dry-run", "--shell", "./missing-just-6-shell"])
    .justfile("foo:\n  echo {{shell('exit 7')}}\n")
    .stderr("echo shell(\"exit 7\")\n")
    .success();
}

#[test]
fn dry_run_quotes_empty_and_spaced_arguments_in_order() {
  Test::new()
    .justfile("foo:\n  echo {{shell('exit 7', '', 'two words', 'last')}}\n")
    .arg("--dry-run")
    .stderr("echo shell(\"exit 7\", \"\", \"two words\", \"last\")\n")
    .success();
}

#[test]
fn dry_run_escapes_backslash_and_control_characters() {
  Test::new()
    .justfile(r#"foo:
  echo {{shell('exit 7', "a\\b\nc\rd\te\"")}}
"#)
    .arg("--dry-run")
    .stderr("echo shell(\"exit 7\", \"a\\\\b\\nc\\rd\\te\\\"\")\n")
    .success();
}

#[test]
fn normal_shell_execution_retains_output_arguments_and_side_effects() {
  Test::new()
    .justfile(
      r#"value := shell('touch normal-marker; printf "%s|%s\n\n" "$1" "$2"', 'two words', 'last')
foo:
  @echo '{{value}}'
"#,
    )
    .stdout("two words|last\n\n")
    .unindent_stdout(false)
    .expect_file("normal-marker", "")
    .success();
}

#[test]
fn normal_shell_failure_is_still_reported() {
  Test::new()
    .justfile("value := shell('exit 7')\nfoo:\n  echo {{value}}\n")
    .stderr_regex(r"error: [Cc]all to function `shell` failed: [Pp]rocess exited with status code 7\n.*")
    .failure();
}

#[test]
fn dry_run_backticks_are_unevaluated_and_have_no_side_effects() {
  let output = Test::new()
    .justfile(
      "value := `touch assignment-marker; printf value`\nfoo:\n  echo {{value}}\n  echo {{`touch interpolation-marker; printf other`}}\n",
    )
    .arg("--dry-run")
    .stderr("echo `touch assignment-marker; printf value`\necho `touch interpolation-marker; printf other`\n")
    .success();
  assert!(!output.tempdir.path().join("assignment-marker").exists());
  assert!(!output.tempdir.path().join("interpolation-marker").exists());
}

#[test]
fn normal_backticks_still_execute_and_preserve_newline_handling() {
  Test::new()
    .justfile("value := `touch normal-marker; printf 'value\\n\\n'`\nfoo:\n  @echo '{{value}}'\n")
    .stdout("value\n\n")
    .unindent_stdout(false)
    .expect_file("normal-marker", "")
    .success();
}
