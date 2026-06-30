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
