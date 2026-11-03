use super::*;

#[test]
fn dry_run_shell_does_not_change_files() {
  Test::new()
    .write("sentinel", "original")
    .justfile(
      "
        foo:
          echo {{ shell('printf changed > sentinel; printf result') }}
      ",
    )
    .arg("--dry-run")
    .stderr("echo shell(\"printf changed > sentinel; printf result\")\n")
    .expect_file("sentinel", "original")
    .success();
}

#[test]
fn dry_run_shell_in_assignment_evaluation() {
  Test::new()
    .justfile("x := shell('exit 9')")
    .args(["--dry-run", "--evaluate", "x"])
    .stdout("shell(\"exit 9\")")
    .success();
}

#[test]
fn dry_run_quotes_empty_backslash_control_and_unicode_arguments() {
  Test::new()
    .justfile(
      r#"
        foo:
          echo {{ shell('exit 1', '', 'C:\tmp\file', "\t\n\r", "a\"b'c", '雪') }}
      "#,
    )
    .arg("--dry-run")
    .stderr("echo shell(\"exit 1\", \"\", \"C:\\\\tmp\\\\file\", \"\\t\\n\\r\", \"a\\\"b'c\", \"雪\")\n")
    .success();
}

#[test]
fn dry_run_quotes_command_argument() {
  Test::new()
    .justfile(
      r#"
        foo:
          echo {{ shell('printf "%s" "C:\tmp"') }}
      "#,
    )
    .arg("--dry-run")
    .stderr("echo shell(\"printf \\\"%s\\\" \\\"C:\\\\tmp\\\"\")\n")
    .success();
}

#[test]
fn dry_run_nested_shell_calls_are_not_executed() {
  Test::new()
    .justfile(
      "
        foo:
          echo {{ shell('printf %s', shell('exit 1')) }}
      ",
    )
    .arg("--dry-run")
    .stderr("echo shell(\"printf %s\", \"shell(\\\"exit 1\\\")\")\n")
    .success();
}

#[test]
fn dry_run_does_not_launch_configured_shell() {
  Test::new()
    .shell(false)
    .justfile(
      "
        set shell := ['/just-6-nonexistent-shell', '-c']
        foo:
          echo {{ shell('exit 1') }}
      ",
    )
    .arg("--dry-run")
    .stderr("echo shell(\"exit 1\")\n")
    .success();
}

#[test]
fn ordinary_shell_execution_preserves_side_effects_arguments_and_trimming() {
  Test::new()
    .write("sentinel", "original")
    .justfile(
      r#"
        value := shell('printf changed > sentinel; printf "%s\n\n" "$1"', 'ok')
        foo:
          @echo {{ value }}
      "#,
    )
    .stdout("ok\n")
    .expect_file("sentinel", "changed")
    .success();
}

#[test]
fn dry_run_backtick_behavior_is_preserved() {
  Test::new()
    .write("sentinel", "original")
    .justfile(
      "
        value := `printf changed > sentinel; printf ok`
        foo:
          echo {{ value }}
      ",
    )
    .arg("--dry-run")
    .stderr("echo `printf changed > sentinel; printf ok`\n")
    .expect_file("sentinel", "original")
    .success();
}

#[test]
fn ordinary_backtick_execution_is_preserved() {
  Test::new()
    .justfile(
      r#"
        value := `printf 'ok\n\n'`
        foo:
          @echo {{ value }}
      "#,
    )
    .stdout("ok\n")
    .success();
}
