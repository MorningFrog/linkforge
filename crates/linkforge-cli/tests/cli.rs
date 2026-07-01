use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn hardlink_and_same_file_commands_work() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    let hardlink = temp.path().join("hard.txt");
    fs::write(&source, "hello").unwrap();

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "hardlink",
            source.to_str().unwrap(),
            hardlink.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created hard link"));

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "same-file",
            source.to_str().unwrap(),
            hardlink.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Same file"));
}

#[test]
fn link_count_command_reports_count() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    fs::write(&source, "hello").unwrap();

    Command::cargo_bin("linkforge")
        .unwrap()
        .args(["link-count", source.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Link count"));
}

#[test]
fn existing_target_fails_without_force() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.txt");
    let target = temp.path().join("target.txt");
    fs::write(&source, "source").unwrap();
    fs::write(&target, "target").unwrap();

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "hardlink",
            source.to_str().unwrap(),
            target.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("target already exists"));
}

#[test]
fn batch_hardlink_creates_multiple_file_links() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    let first = temp.path().join("first.txt");
    let second = temp.path().join("second.txt");
    fs::create_dir(&target).unwrap();
    fs::write(&first, "first").unwrap();
    fs::write(&second, "second").unwrap();

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "batch-hardlink",
            "--target-dir",
            target.to_str().unwrap(),
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created file hard link"));

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "same-file",
            first.to_str().unwrap(),
            target.join("first.txt").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Same file"));
}

#[test]
fn batch_hardlink_creates_directory_tree() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    let source = temp.path().join("source-dir");
    let nested = source.join("nested");
    fs::create_dir(&target).unwrap();
    fs::create_dir(&source).unwrap();
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("nested.txt"), "nested").unwrap();

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "batch-hardlink",
            "--target-dir",
            target.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created hard-link tree"));

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "same-file",
            nested.join("nested.txt").to_str().unwrap(),
            target
                .join("source-dir")
                .join("nested")
                .join("nested.txt")
                .to_str()
                .unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Same file"));
}

#[test]
fn batch_symlink_dry_run_does_not_create_target() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    let source = temp.path().join("source.txt");
    fs::create_dir(&target).unwrap();
    fs::write(&source, "source").unwrap();

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "batch-symlink",
            "--target-dir",
            target.to_str().unwrap(),
            "--dry-run",
            source.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run complete"));

    assert!(!target.join("source.txt").exists());
}

#[test]
fn batch_rename_and_skip_conflicts() {
    let temp = tempfile::tempdir().unwrap();
    let first_dir = temp.path().join("first");
    let second_dir = temp.path().join("second");
    let target = temp.path().join("target");
    fs::create_dir(&first_dir).unwrap();
    fs::create_dir(&second_dir).unwrap();
    fs::create_dir(&target).unwrap();
    let first = first_dir.join("same.txt");
    let second = second_dir.join("same.txt");
    fs::write(&first, "first").unwrap();
    fs::write(&second, "second").unwrap();
    fs::write(target.join("same.txt"), "existing").unwrap();

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "batch-hardlink",
            "--target-dir",
            target.to_str().unwrap(),
            "--on-conflict",
            "rename",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("same - Link.txt"))
        .stdout(predicate::str::contains("same - Link (2).txt"));

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "batch-hardlink",
            "--target-dir",
            target.to_str().unwrap(),
            "--on-conflict",
            "skip",
            first.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("target already exists"));
}

#[test]
fn batch_conflict_fails_by_default() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    let source = temp.path().join("same.txt");
    fs::create_dir(&target).unwrap();
    fs::write(&source, "source").unwrap();
    fs::write(target.join("same.txt"), "existing").unwrap();

    Command::cargo_bin("linkforge")
        .unwrap()
        .args([
            "batch-hardlink",
            "--target-dir",
            target.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Conflict"))
        .stderr(predicate::str::contains("batch preflight found conflicts"));
}

#[test]
fn help_command_lists_commands() {
    Command::cargo_bin("linkforge")
        .unwrap()
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("symlink"))
        .stdout(predicate::str::contains("batch-symlink"))
        .stdout(predicate::str::contains("completions"));
}

#[test]
fn help_for_symlink_describes_arguments_and_force() {
    Command::cargo_bin("linkforge")
        .unwrap()
        .args(["help", "symlink"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SOURCE"))
        .stdout(predicate::str::contains("LINK"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn completion_scripts_are_generated_for_supported_shells() {
    let cases = [
        ("powershell", "Register-ArgumentCompleter"),
        ("bash", "_linkforge"),
        ("zsh", "#compdef linkforge"),
        ("fish", "complete -c linkforge"),
    ];

    for (shell, expected) in cases {
        Command::cargo_bin("linkforge")
            .unwrap()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains(expected));
    }
}

#[test]
fn unsupported_completion_shell_fails() {
    Command::cargo_bin("linkforge")
        .unwrap()
        .args(["completions", "elvish"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}
