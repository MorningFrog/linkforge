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
fn help_command_lists_commands() {
    Command::cargo_bin("linkforge")
        .unwrap()
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("symlink"))
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
