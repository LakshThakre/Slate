use std::fs;
use std::process::{Command as StdCommand, Stdio};
use std::thread;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_add_list_done_rm_flow() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");

    // 1. Add "Buy fresh milk"
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("add")
        .arg("Buy fresh milk")
        .assert()
        .success()
        .stdout("Added: Buy fresh milk\n");

    // 2. Add "Review PR" with --quiet
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("add")
        .arg("-q")
        .arg("Review PR")
        .assert()
        .success()
        .stdout("");

    // 3. List
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Buy fresh milk").and(predicate::str::contains("Review PR")),
        );

    // 4. Mark 1 done
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("done")
        .arg("1")
        .assert()
        .success();

    // 5. List shows task 1 is done
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("[x]  Buy fresh milk")
                .and(predicate::str::contains("[ ]  Review PR")),
        );

    // 6. today shows 1 of 2 done
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("today")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 of 2 done"));

    // 7. rm 1
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("rm")
        .arg("1")
        .assert()
        .success();

    // 8. List shows only task 2 remains
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Review PR")
                .and(predicate::str::contains("Buy fresh milk").not()),
        );
}

#[test]
fn test_exit_codes() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");

    // Exit 0: success
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("path")
        .assert()
        .code(0);

    // Exit 1: corrupted JSON file
    let bad_file = dir.path().join("bad.json");
    fs::write(&bad_file, "INVALID JSON DATA").unwrap();
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&bad_file)
        .arg("list")
        .assert()
        .code(1)
        .stderr(predicate::str::starts_with("slate: "));
    // Assert bad file is untouched
    assert_eq!(fs::read_to_string(&bad_file).unwrap(), "INVALID JSON DATA");

    // Exit 2: usage error (empty task text)
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("add")
        .arg("   ")
        .assert()
        .code(2)
        .stderr(predicate::str::starts_with("slate: "));

    // Exit 2: usage error (invalid date)
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("list")
        .arg("--date")
        .arg("not-a-valid-date")
        .assert()
        .code(2)
        .stderr(predicate::str::starts_with("slate: "));

    // Exit 3: no such task (out of range index)
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("done")
        .arg("5")
        .assert()
        .code(3)
        .stderr(predicate::str::starts_with("slate: "));

    // Exit 3: no such task (--match found nothing)
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("rm")
        .arg("--match")
        .arg("nonexistent")
        .assert()
        .code(3)
        .stderr(predicate::str::starts_with("slate: "));
}

#[test]
fn test_ambiguous_match_lists_candidates_on_stderr_with_exit_3() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");

    // Add two matching tasks
    Command::cargo_bin("slate")
        .unwrap()
        .arg("--data-file")
        .arg(&data_file)
        .arg("add")
        .arg("Feed cat")
        .assert()
        .success();

    Command::cargo_bin("slate")
        .unwrap()
        .arg("--data-file")
        .arg(&data_file)
        .arg("add")
        .arg("Buy cat food")
        .assert()
        .success();

    // Match "cat" is ambiguous
    Command::cargo_bin("slate")
        .unwrap()
        .arg("--data-file")
        .arg(&data_file)
        .arg("done")
        .arg("--match")
        .arg("cat")
        .assert()
        .code(3)
        .stderr(
            predicate::str::contains("multiple tasks match 'cat':")
                .and(predicate::str::contains("1  Feed cat"))
                .and(predicate::str::contains("2  Buy cat food")),
        );
}

#[test]
fn test_list_nonexistent_file_returns_no_tasks_and_exit_0() {
    let dir = tempdir().unwrap();
    let missing_file = dir.path().join("missing.json");

    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&missing_file)
        .arg("list")
        .assert()
        .code(0)
        .stdout("");

    // File was not created on read
    assert!(!missing_file.exists());
}

#[test]
fn test_today_empty_prints_0_of_0_done() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");

    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("today")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("0 of 0 done"));
}

#[test]
fn test_no_subcommand_non_tty_guard() {
    // When stdout is redirected to piped output, running `slate` without subcommand fails with exit 1
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.assert()
        .code(1)
        .stderr("slate: not a terminal (try \"slate list\")\n");
}

#[test]
fn test_slate_data_env_var_precedence() {
    let dir = tempdir().unwrap();
    let env_file = dir.path().join("env_todos.json");
    let flag_file = dir.path().join("flag_todos.json");

    // 1. Set SLATE_DATA env var
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.env("SLATE_DATA", &env_file)
        .arg("path")
        .assert()
        .success()
        .stdout(format!("{}\n", env_file.display()));

    // 2. Flag overrides SLATE_DATA
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.env("SLATE_DATA", &env_file)
        .arg("--data-file")
        .arg(&flag_file)
        .arg("path")
        .assert()
        .success()
        .stdout(format!("{}\n", flag_file.display()));
}

#[test]
fn test_piped_output_contains_no_ansi_escapes() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");

    Command::cargo_bin("slate")
        .unwrap()
        .arg("--data-file")
        .arg(&data_file)
        .arg("add")
        .arg("Test task")
        .assert()
        .success();

    let output = Command::cargo_bin("slate")
        .unwrap()
        .arg("--data-file")
        .arg(&data_file)
        .arg("list")
        .output()
        .unwrap();

    let stdout_str = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout_str.contains("\x1b["),
        "output contained ANSI escapes"
    );
}

#[test]
fn test_legacy_v0_todos_json_loads_unchanged() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");

    // Legacy v0 json: simple array of objects without version wrapper
    let legacy_json = r#"[
        {"id": 1, "text": "Legacy task 1", "created_date": "2026-10-25", "done": true},
        {"id": 2, "text": "Legacy task 2", "created_date": "2026-10-25", "done": false}
    ]"#;
    fs::write(&data_file, legacy_json).unwrap();

    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--data-file")
        .arg(&data_file)
        .arg("list")
        .arg("--all")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("2026-10-25")
                .and(predicate::str::contains("[x]  Legacy task 1"))
                .and(predicate::str::contains("[ ]  Legacy task 2")),
        );
}

#[test]
fn test_broken_pipe_quiet_exit() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");

    // Add several tasks
    for i in 1..=20 {
        Command::cargo_bin("slate")
            .unwrap()
            .arg("--data-file")
            .arg(&data_file)
            .arg("add")
            .arg(format!("Task {i}"))
            .assert()
            .success();
    }

    let bin_path = assert_cmd::cargo::cargo_bin("slate");
    let mut child = StdCommand::new(bin_path)
        .arg("--data-file")
        .arg(&data_file)
        .arg("list")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Close stdout pipe immediately to trigger broken pipe
    drop(child.stdout.take());

    let output = child.wait_with_output().unwrap();
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr_str.contains("panic"),
        "panicked on broken pipe: {stderr_str}"
    );
}

#[test]
fn test_concurrency_100_parallel_adds() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("concurrent_todos.json");

    let file_path = data_file.clone();
    let t1 = thread::spawn(move || {
        for i in 0..50 {
            Command::cargo_bin("slate")
                .unwrap()
                .arg("--data-file")
                .arg(&file_path)
                .arg("add")
                .arg(format!("Thread 1 task {i}"))
                .assert()
                .success();
        }
    });

    let file_path2 = data_file.clone();
    let t2 = thread::spawn(move || {
        for i in 0..50 {
            Command::cargo_bin("slate")
                .unwrap()
                .arg("--data-file")
                .arg(&file_path2)
                .arg("add")
                .arg(format!("Thread 2 task {i}"))
                .assert()
                .success();
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();

    // Verify 100 tasks exist and JSON is valid
    let loaded = slate::storage::load_todos(&data_file).unwrap();
    assert_eq!(loaded.todos.len(), 100);
}
