use std::fs;
use std::io::{self, Write};
use std::thread;

use assert_cmd::Command;
use chrono::NaiveDate;
use predicates::prelude::*;
use slate::commands::{
    handle_add, handle_add_with_reader, handle_done, handle_list, handle_rm, handle_today, CmdError,
};
use slate::config::Config;
use slate::output::SafeWriter;
use tempfile::tempdir;

// ==================== BINARY TESTS (NO DATA READ/WRITE) ====================

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("slate "));
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: slate"));
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
fn test_usage_error_empty_task_text() {
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("add")
        .arg("   ")
        .assert()
        .code(2)
        .stderr(predicate::str::starts_with("slate: "));
}

#[test]
fn test_usage_error_invalid_date() {
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("list")
        .arg("--date")
        .arg("not-a-valid-date")
        .assert()
        .code(2)
        .stderr(predicate::str::starts_with("slate: "));
}

#[test]
fn test_usage_error_unknown_subcommand() {
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("unknown-subcommand").assert().code(2);
}

#[test]
fn test_usage_error_unknown_option() {
    let mut cmd = Command::cargo_bin("slate").unwrap();
    cmd.arg("--unknown-flag").assert().code(2);
}

// ==================== HANDLER-LEVEL TESTS (INJECTED PATHS) ====================

#[test]
fn test_handler_add_list_done_rm_flow() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();

    // 1. Add "Buy fresh milk"
    let mut out = Vec::new();
    handle_add(
        vec!["Buy fresh milk".to_string()],
        None,
        false,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8_lossy(&out), "Added: Buy fresh milk\n");
    out.clear();

    // 2. Add "Review PR" with quiet = true
    handle_add(
        vec!["Review PR".to_string()],
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();
    assert!(out.is_empty());

    // 3. List shows both tasks
    handle_list(
        None, false, false, None, &data_file, today, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(list_str.contains("Buy fresh milk"));
    assert!(list_str.contains("Review PR"));
    out.clear();

    // 4. Mark task 1 done
    handle_done(vec![1], None, None, &data_file, today).unwrap();

    // 5. List shows task 1 is done
    handle_list(
        None, false, false, None, &data_file, today, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(
        list_str.contains("Buy fresh milk") && list_str.contains("done"),
        "expected done marker for task 1, got: {list_str}"
    );
    assert!(
        list_str.contains("Review PR"),
        "expected task 2, got: {list_str}"
    );
    out.clear();

    // 6. Today shows 1 of 2 done
    handle_today(&data_file, today, &config, &mut out).unwrap();
    let today_str = String::from_utf8_lossy(&out);
    assert!(today_str.contains("1 of 2 done"));
    out.clear();

    // 7. Delete task 1
    handle_rm(vec![1], None, None, &data_file, today).unwrap();

    // 8. List shows only task 2 remains
    handle_list(
        None, false, false, None, &data_file, today, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(list_str.contains("Review PR"));
    assert!(!list_str.contains("Buy fresh milk"));
}

#[test]
fn test_handler_corrupted_json_exit_code_1() {
    let dir = tempdir().unwrap();
    let bad_file = dir.path().join("bad.json");
    fs::write(&bad_file, "INVALID JSON DATA").unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();
    let mut out = Vec::new();

    let res = handle_list(
        None, false, false, None, &bad_file, today, &config, &mut out,
    );
    match res {
        Err(err) => {
            assert_eq!(err.exit_code(), 1);
            assert!(err.message().contains("corrupted") || err.message().contains("expected"));
        }
        Ok(_) => panic!("expected runtime error for corrupted JSON"),
    }

    // Corrupted file was not overwritten
    assert_eq!(fs::read_to_string(&bad_file).unwrap(), "INVALID JSON DATA");
}

#[test]
fn test_handler_exit_code_3_out_of_range() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

    let res = handle_done(vec![5], None, None, &data_file, today);
    match res {
        Err(err) => {
            assert_eq!(err.exit_code(), 3);
            assert!(err.message().contains("out of range"));
        }
        Ok(_) => panic!("expected not found error"),
    }
}

#[test]
fn test_handler_exit_code_3_match_not_found() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

    let res = handle_rm(
        vec![],
        None,
        Some("nonexistent".to_string()),
        &data_file,
        today,
    );
    match res {
        Err(err) => {
            assert_eq!(err.exit_code(), 3);
            assert!(err.message().contains("no task matches 'nonexistent'"));
        }
        Ok(_) => panic!("expected not found error"),
    }
}

#[test]
fn test_handler_ambiguous_match_lists_candidates_exit_code_3() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let mut out = Vec::new();

    handle_add(
        vec!["Feed cat".to_string()],
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();
    handle_add(
        vec!["Buy cat food".to_string()],
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    let res = handle_done(vec![], None, Some("cat".to_string()), &data_file, today);
    match res {
        Err(CmdError::NotFound(msg)) => {
            assert!(msg.contains("multiple tasks match 'cat':"));
            assert!(msg.contains("1  Feed cat"));
            assert!(msg.contains("2  Buy cat food"));
        }
        other => panic!("expected ambiguous match NotFound, got: {other:?}"),
    }
}

#[test]
fn test_handler_list_nonexistent_file_returns_no_tasks_and_exit_0() {
    let dir = tempdir().unwrap();
    let missing_file = dir.path().join("missing.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();
    let mut out = Vec::new();

    let res = handle_list(
        None,
        false,
        false,
        None,
        &missing_file,
        today,
        &config,
        &mut out,
    );
    assert!(res.is_ok());
    assert!(out.is_empty());
    assert!(!missing_file.exists());
}

#[test]
fn test_handler_today_empty_prints_0_of_0_done() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();
    let mut out = Vec::new();

    let res = handle_today(&data_file, today, &config, &mut out);
    assert!(res.is_ok());
    let today_str = String::from_utf8_lossy(&out);
    assert!(today_str.contains("0 of 0 done"));
}

#[test]
fn test_handler_piped_output_contains_no_ansi_escapes() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();
    let mut out = Vec::new();

    std::env::set_var("NO_COLOR", "1");
    handle_add(
        vec!["Test task".to_string()],
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    handle_list(
        None, false, false, None, &data_file, today, &config, &mut out,
    )
    .unwrap();
    std::env::remove_var("NO_COLOR");

    let stdout_str = String::from_utf8_lossy(&out);
    assert!(
        !stdout_str.contains("\x1b["),
        "output contained ANSI escapes: {stdout_str}"
    );
}

#[test]
fn test_handler_legacy_v0_todos_json_loads_unchanged() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();

    let legacy_json = r#"[
        {"id": 1, "text": "Legacy task 1", "created_date": "2026-10-25", "done": true},
        {"id": 2, "text": "Legacy task 2", "created_date": "2026-10-25", "done": false}
    ]"#;
    fs::write(&data_file, legacy_json).unwrap();

    let mut out = Vec::new();
    handle_list(
        None, true, false, None, &data_file, today, &config, &mut out,
    )
    .unwrap();
    let stdout_str = String::from_utf8_lossy(&out);
    assert!(stdout_str.contains("2026-10-25"));
    assert!(
        stdout_str.contains("Legacy task 1") && stdout_str.contains("done"),
        "expected done legacy task 1: {stdout_str}"
    );
    assert!(
        stdout_str.contains("Legacy task 2"),
        "expected pending legacy task 2: {stdout_str}"
    );
}

#[test]
fn test_handler_broken_pipe_quiet_exit() {
    struct BrokenPipeWriter;
    impl Write for BrokenPipeWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"))
        }
    }

    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();

    let mut dummy_out = Vec::new();
    for i in 1..=10 {
        handle_add(
            vec![format!("Task {i}")],
            None,
            true,
            &data_file,
            today,
            &mut dummy_out,
        )
        .unwrap();
    }

    let mut safe_writer = SafeWriter::new(BrokenPipeWriter);
    let res = handle_list(
        None,
        false,
        false,
        None,
        &data_file,
        today,
        &config,
        &mut safe_writer,
    );
    assert!(res.is_ok(), "handler failed on broken pipe: {res:?}");
    assert!(safe_writer.is_broken_pipe());
}

#[test]
fn test_handler_concurrency_100_parallel_adds() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("concurrent_todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

    let file_path = data_file.clone();
    let t1 = thread::spawn(move || {
        for i in 0..50 {
            handle_add_with_reader(
                vec![format!("Thread 1 task {i}")],
                None,
                true,
                &file_path,
                today,
                &mut io::sink(),
                io::empty(),
            )
            .unwrap();
        }
    });

    let file_path2 = data_file.clone();
    let t2 = thread::spawn(move || {
        for i in 0..50 {
            handle_add_with_reader(
                vec![format!("Thread 2 task {i}")],
                None,
                true,
                &file_path2,
                today,
                &mut io::sink(),
                io::empty(),
            )
            .unwrap();
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();

    // Verify 100 tasks exist and JSON is valid
    let loaded = slate::storage::load_todos(&data_file).unwrap();
    assert_eq!(loaded.todos.len(), 100);
}

#[test]
fn test_cli_add_quick_add_syntax_and_errors() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let mut out = Vec::new();

    // 1. Success with tokens
    handle_add(
        vec!["Submit tax return @tomorrow !! #finance #taxes".to_string()],
        None,
        false,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8_lossy(&out), "Added: Submit tax return\n");
    out.clear();

    let loaded = slate::storage::load_todos(&data_file).unwrap().todos;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].text, "Submit tax return");
    assert_eq!(loaded[0].priority, 2);
    assert_eq!(loaded[0].tags, vec!["finance", "taxes"]);
    assert_eq!(
        loaded[0].created_date,
        NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
    );

    // 2. Error: Past date -> exit code 2 (CmdError::Usage)
    let err_past = handle_add(
        vec!["Old task @2026-10-20".to_string()],
        None,
        false,
        &data_file,
        today,
        &mut out,
    );
    assert!(matches!(err_past, Err(CmdError::Usage(_))));

    // 3. Error: Empty text after stripping tokens -> exit code 2 (CmdError::Usage)
    let err_empty = handle_add(
        vec!["@tomorrow !! #tag".to_string()],
        None,
        false,
        &data_file,
        today,
        &mut out,
    );
    assert!(matches!(err_empty, Err(CmdError::Usage(_))));
}

#[test]
fn test_cli_subtasks_listing_and_operations() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();
    let mut out = Vec::new();

    // 1. Add parent task
    handle_add(
        vec!["Parent task".to_string()],
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    // Add subtask directly to storage
    slate::storage::try_load_modify_save(&data_file, |data| {
        let id = data.next_id;
        data.next_id += 1;
        data.todos.push(slate::model::Todo {
            id,
            text: "Child task".to_string(),
            created_date: today,
            done: false,
            parent: Some(1),
            ..Default::default()
        });
        Ok::<(), CmdError>(())
    })
    .unwrap();

    // 2. List shows "1" for parent and "1.1" indented for subtask
    handle_list(
        None, false, false, None, &data_file, today, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(
        list_str.contains("Parent task") && (list_str.contains("○") || list_str.contains("[ ]"))
    );
    assert!(list_str.contains("1.1") && list_str.contains("Child task"));
    out.clear();

    // 3. Mark subtask done using handle_done_str with "1.1"
    slate::commands::handle_done_str(&["1.1".to_string()], None, None, &data_file, today).unwrap();

    let loaded = slate::storage::load_todos(&data_file).unwrap().todos;
    assert!(!loaded[0].done); // Parent still unfinished
    assert!(loaded[1].done); // Child is done

    // 4. Mark parent done -> completes both
    slate::commands::handle_done(vec![1], None, None, &data_file, today).unwrap();
    let loaded = slate::storage::load_todos(&data_file).unwrap().todos;
    assert!(loaded[0].done);
    assert!(loaded[1].done);

    // 5. Undone child -> reopens parent
    slate::commands::handle_undone_str(&["1.1".to_string()], None, None, &data_file, today)
        .unwrap();
    let loaded = slate::storage::load_todos(&data_file).unwrap().todos;
    assert!(!loaded[0].done); // Parent reopened!
    assert!(!loaded[1].done);

    // 6. Delete subtask with rm 1.1
    slate::commands::handle_rm_str(&["1.1".to_string()], None, None, &data_file, today).unwrap();
    let loaded = slate::storage::load_todos(&data_file).unwrap().todos;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, 1);
}

#[test]
fn test_cli_recurring_tasks_addition_and_materialization() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let monday = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    let tuesday = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
    let config = Config::default();
    let mut out = Vec::new();

    // 1. Add daily task via quick-add CLI
    handle_add(
        vec!["Daily standup @daily".to_string()],
        None,
        true,
        &data_file,
        monday,
        &mut out,
    )
    .unwrap();
    out.clear();

    let loaded = slate::storage::load_todos(&data_file).unwrap();
    assert_eq!(loaded.todos.len(), 1);
    assert_eq!(loaded.todos[0].text, "Daily standup");
    assert_eq!(loaded.recurring.len(), 1);
    assert_eq!(
        loaded.recurring[0].rule,
        slate::model::RecurrenceRule::Daily
    );

    // 2. Listing on Tuesday materializes Tuesday instance
    handle_list(
        None, false, false, None, &data_file, tuesday, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(list_str.contains("Daily standup"));
    out.clear();

    let loaded_tue = slate::storage::load_todos(&data_file).unwrap();
    assert_eq!(loaded_tue.todos.len(), 2);
    assert_eq!(loaded_tue.todos[0].created_date, monday);
    assert_eq!(loaded_tue.todos[1].created_date, tuesday);
}
