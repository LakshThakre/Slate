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
        None, false, false, None, None, None, false, &data_file, today, &config, &mut out,
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
        None, false, false, None, None, None, false, &data_file, today, &config, &mut out,
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
        None, false, false, None, None, None, false, &data_file, today, &config, &mut out,
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
        None, false, false, None, None, None, false, &bad_file, today, &config, &mut out,
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
        None,
        None,
        false,
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
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    handle_list(
        None, false, false, None, None, None, false, &data_file, today, &config, &mut out,
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
        None, true, false, None, None, None, false, &data_file, today, &config, &mut out,
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
        None,
        None,
        false,
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

    // Verify 100 tasks exist, all IDs are unique, and next_id >= 101
    let loaded = slate::storage::load_todos(&data_file).unwrap();
    assert_eq!(loaded.todos.len(), 100);
    let ids: std::collections::HashSet<u64> = loaded.todos.iter().map(|t| t.id).collect();
    assert_eq!(ids.len(), 100, "all 100 task IDs must be unique");
    assert!(loaded.next_id >= 101, "next_id must be at least 101");
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
        None,
        false,
        &data_file,
        today,
        &mut out,
    );
    assert!(matches!(err_empty, Err(CmdError::Usage(_))));
}

#[test]
fn test_cli_add_parent_subtask() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let mut out = Vec::new();

    // 1. Add top-level parent task
    handle_add(
        vec!["Parent task".to_string()],
        None,
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    // 2. Add subtask via --parent 1
    handle_add(
        vec!["Subtask one ! #child".to_string()],
        None,
        Some("1".to_string()),
        false,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8_lossy(&out), "Added: Subtask one\n");
    out.clear();

    let loaded = slate::storage::load_todos(&data_file).unwrap().todos;
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].text, "Parent task");
    assert_eq!(loaded[1].text, "Subtask one");
    assert_eq!(loaded[1].parent, Some(loaded[0].id));
    assert_eq!(loaded[1].created_date, loaded[0].created_date);
    assert_eq!(loaded[1].priority, 1);
    assert_eq!(loaded[1].tags, vec!["child"]);

    // 3. Subtask cannot have its own @date or @recurrence token
    let res_date = handle_add(
        vec!["Invalid subtask @tomorrow".to_string()],
        None,
        Some("1".to_string()),
        true,
        &data_file,
        today,
        &mut out,
    );
    assert!(matches!(res_date, Err(CmdError::Usage(_))));

    let res_rec = handle_add(
        vec!["Invalid subtask @daily".to_string()],
        None,
        Some("1".to_string()),
        true,
        &data_file,
        today,
        &mut out,
    );
    assert!(matches!(res_rec, Err(CmdError::Usage(_))));

    // 4. Subtask cannot be nested under another subtask (one level only)
    let res_nested = handle_add(
        vec!["Deep child".to_string()],
        None,
        Some("1.1".to_string()),
        true,
        &data_file,
        today,
        &mut out,
    );
    assert!(matches!(res_nested, Err(CmdError::Usage(_))));

    // 5. Parent not found
    let res_not_found = handle_add(
        vec!["Orphan".to_string()],
        None,
        Some("99".to_string()),
        true,
        &data_file,
        today,
        &mut out,
    );
    assert!(matches!(res_not_found, Err(CmdError::NotFound(_))));
}

#[test]
fn test_cli_date_vs_token_date_conflict() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let mut out = Vec::new();

    // --date conflict with @date token returns CmdError::Usage (exit code 2)
    let res = handle_add(
        vec!["Task @2026-11-05".to_string()],
        Some("2026-11-01".to_string()),
        None,
        true,
        &data_file,
        today,
        &mut out,
    );
    match res {
        Err(CmdError::Usage(msg)) => {
            assert!(msg.contains("conflicting dates"));
        }
        other => panic!("expected conflicting date usage error, got {other:?}"),
    }

    // Matching dates succeed
    let res_ok = handle_add(
        vec!["Task @2026-11-01".to_string()],
        Some("2026-11-01".to_string()),
        None,
        true,
        &data_file,
        today,
        &mut out,
    );
    assert!(res_ok.is_ok());
}

#[test]
fn test_cli_list_filters_and_upcoming() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();
    let mut out = Vec::new();

    // Add tasks
    handle_add(
        vec!["Work task ! #work".to_string()],
        None,
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();
    handle_add(
        vec!["Home task !! #home".to_string()],
        None,
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();
    handle_add(
        vec!["Future task @tomorrow #future".to_string()],
        None,
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    // 1. Filter by tag: #work
    handle_list(
        None,
        false,
        false,
        None,
        Some("work".to_string()),
        None,
        false,
        &data_file,
        today,
        &config,
        &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(list_str.contains("Work task"));
    assert!(!list_str.contains("Home task"));
    out.clear();

    // 2. Filter by tag with hash prefix: #home
    handle_list(
        None,
        false,
        false,
        None,
        Some("#home".to_string()),
        None,
        false,
        &data_file,
        today,
        &config,
        &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(!list_str.contains("Work task"));
    assert!(list_str.contains("Home task"));
    out.clear();

    // 3. Filter by min-priority 2 (!!)
    handle_list(
        None,
        false,
        false,
        None,
        None,
        Some(2),
        false,
        &data_file,
        today,
        &config,
        &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(!list_str.contains("Work task")); // priority 1
    assert!(list_str.contains("Home task")); // priority 2
    out.clear();

    // 4. Invalid min-priority 3
    let res_inv = handle_list(
        None,
        false,
        false,
        None,
        None,
        Some(3),
        false,
        &data_file,
        today,
        &config,
        &mut out,
    );
    assert!(matches!(res_inv, Err(CmdError::Usage(_))));

    // 5. Upcoming view (shows future tasks)
    handle_list(
        None, false, false, None, None, None, true, &data_file, today, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(list_str.contains("Future task"));
    assert!(!list_str.contains("Work task"));
    out.clear();

    // 6. Conflicting upcoming flags
    let res_up_all = handle_list(
        None, true, false, None, None, None, true, &data_file, today, &config, &mut out,
    );
    assert!(matches!(res_up_all, Err(CmdError::Usage(_))));

    let res_up_date = handle_list(
        Some("today".to_string()),
        false,
        false,
        None,
        None,
        None,
        true,
        &data_file,
        today,
        &config,
        &mut out,
    );
    assert!(matches!(res_up_date, Err(CmdError::Usage(_))));
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
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    // Add subtask via --parent
    handle_add(
        vec!["Child task".to_string()],
        None,
        Some("1".to_string()),
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    // 2. List shows "1" for parent with subtask progress and "1.1" indented for subtask
    handle_list(
        None, false, false, None, None, None, false, &data_file, today, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(
        list_str.contains("Parent task") && (list_str.contains("○") || list_str.contains("[ ]"))
    );
    assert!(
        list_str.contains("0/1"),
        "parent must display subtask progress 0/1"
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
fn test_cli_read_handlers_do_not_modify_file() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let monday = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
    let tuesday = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
    let config = Config::default();
    let mut out = Vec::new();

    // 1. Add recurring task on Monday
    handle_add(
        vec!["Daily standup @daily".to_string()],
        None,
        None,
        true,
        &data_file,
        monday,
        &mut out,
    )
    .unwrap();
    out.clear();

    let bytes_before = fs::read(&data_file).unwrap();

    // 2. Listing on Tuesday shows materialized instance in output, but disk bytes are identical!
    handle_list(
        None, false, false, None, None, None, false, &data_file, tuesday, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(list_str.contains("Daily standup"));
    out.clear();

    let bytes_after_list = fs::read(&data_file).unwrap();
    assert_eq!(
        bytes_before, bytes_after_list,
        "handle_list must never modify data file on disk"
    );

    // 3. Today command on Tuesday shows task in output, but disk bytes are identical!
    handle_today(&data_file, tuesday, &config, &mut out).unwrap();
    let today_str = String::from_utf8_lossy(&out);
    assert!(today_str.contains("Daily standup"));
    out.clear();

    let bytes_after_today = fs::read(&data_file).unwrap();
    assert_eq!(
        bytes_before, bytes_after_today,
        "handle_today must never modify data file on disk"
    );

    // 4. When a write action occurs on Tuesday (e.g. handle_add), file is updated on disk
    handle_add(
        vec!["Tuesday task".to_string()],
        None,
        None,
        true,
        &data_file,
        tuesday,
        &mut out,
    )
    .unwrap();
    let loaded_after_write = slate::storage::load_todos(&data_file).unwrap();
    assert_eq!(loaded_after_write.todos.len(), 3); // Monday instance, materialized Tuesday instance, and Tuesday task
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

    // 2. Listing on Tuesday materializes Tuesday instance in memory
    handle_list(
        None, false, false, None, None, None, false, &data_file, tuesday, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);
    assert!(list_str.contains("Daily standup"));
    out.clear();

    // Read handlers do not modify disk
    let loaded_tue = slate::storage::load_todos(&data_file).unwrap();
    assert_eq!(loaded_tue.todos.len(), 1);

    // Write handler materializes on disk
    handle_add(
        vec!["New Tuesday task".to_string()],
        None,
        None,
        true,
        &data_file,
        tuesday,
        &mut out,
    )
    .unwrap();
    let loaded_tue_after = slate::storage::load_todos(&data_file).unwrap();
    assert_eq!(loaded_tue_after.todos.len(), 3);
}

#[test]
fn test_cli_human_output_formatting() {
    let dir = tempdir().unwrap();
    let data_file = dir.path().join("todos.json");
    let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
    let config = Config::default();
    let mut out = Vec::new();

    // 1. Add parent task with repeat, priority, tag
    handle_add(
        vec!["Parent task @daily !! #work".to_string()],
        None,
        None,
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    // Add note to parent task
    slate::storage::try_load_modify_save(&data_file, |data| {
        data.todos[0].note = Some("Check slides".to_string());
        Ok::<(), CmdError>(())
    })
    .unwrap();

    // 2. Add subtask with priority and tag
    handle_add(
        vec!["Child step ! #sub".to_string()],
        None,
        Some("1".to_string()),
        true,
        &data_file,
        today,
        &mut out,
    )
    .unwrap();

    // 3. Run handle_list and verify output formatting
    handle_list(
        None, false, false, None, None, None, false, &data_file, today, &config, &mut out,
    )
    .unwrap();
    let list_str = String::from_utf8_lossy(&out);

    // Verify parent row contains:
    // - Checkbox
    // - Repeat glyph (~ or ↻)
    // - Priority marker (!!)
    // - Text
    // - Subtask progress (0/1)
    // - Tag (#work)
    // - Note glyph (= or ≡)
    assert!(list_str.contains("Parent task"));
    assert!(list_str.contains("!!"));
    assert!(list_str.contains("#work"));
    assert!(list_str.contains("0/1"));
    assert!(list_str.contains("~") || list_str.contains("↻"));
    assert!(list_str.contains("=") || list_str.contains("≡"));

    // Verify child row contains:
    // - 1.1 index
    // - Priority marker (!)
    // - Tag (#sub)
    assert!(list_str.contains("1.1"));
    assert!(list_str.contains("Child step"));
    assert!(list_str.contains("!"));
    assert!(list_str.contains("#sub"));
}
