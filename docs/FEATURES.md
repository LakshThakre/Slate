# Slate Features and Architecture Guide

This document is the comprehensive feature manual and architectural specification for Slate. Part 1 documents every user-facing capability, interaction workflow, configuration setting, and command-line interface. Part 2 explains the internal design, data flow, concurrency controls, file persistence model, and testing approach.

## Table of Contents

- [Part 1: Features](#part-1-features)
  - [Views](#views)
    - [Today View](#today-view)
    - [Upcoming View](#upcoming-view)
    - [History View](#history-view)
    - [Summary View](#summary-view)
  - [Working with Tasks](#working-with-tasks)
  - [Quick-Add Syntax](#quick-add-syntax)
  - [Richer Tasks](#richer-tasks)
    - [Priorities and Sorting](#priorities-and-sorting)
    - [Tags and Filtering](#tags-and-filtering)
    - [Notes and the Detail Pane](#notes-and-the-detail-pane)
    - [Subtasks](#subtasks)
    - [Focus Task](#focus-task)
    - [Recurring Tasks](#recurring-tasks)
    - [Multi-Select and Bulk Actions](#multi-select-and-bulk-actions)
    - [Undo and Redo](#undo-and-redo)
  - [Finding Things](#finding-things)
    - [Search Syntax](#search-syntax)
    - [Jump to Date](#jump-to-date)
    - [Streaks and Completion Rules](#streaks-and-completion-rules)
  - [Live Reload](#live-reload)
  - [Command Line](#command-line)
    - [Addressing Tasks](#addressing-tasks)
    - [Date Parameters](#date-parameters)
    - [Non-Interactive and Piped Output](#non-interactive-and-piped-output)
    - [Subcommands](#subcommands)
    - [Exit Codes](#exit-codes)
  - [Customization](#customization)
    - [Configuration Options](#configuration-options)
    - [Themes and Colors](#themes-and-colors)
    - [Monochrome and ASCII Modes](#monochrome-and-ascii-modes)
    - [Progress Bar Styles](#progress-bar-styles)
    - [Keymap Customization](#keymap-customization)
    - [Mouse Support](#mouse-support)
  - [Full Keyboard Reference](#full-keyboard-reference)
- [Part 2: How It Works](#part-2-how-it-works)
  - [Architecture Overview](#architecture-overview)
  - [The Event Loop](#the-event-loop)
  - [State and Data Model](#state-and-data-model)
  - [Storage and Safety](#storage-and-safety)
    - [Atomic File Replacement](#atomic-file-replacement)
    - [Advisory Locking](#advisory-locking)
    - [Rolling Backups](#rolling-backups)
    - [Schema Migrations](#schema-migrations)
    - [Error and Corruption Handling](#error-and-corruption-handling)
  - [Rendering Engine](#rendering-engine)
  - [Keymap and Action Resolution](#keymap-and-action-resolution)
  - [Undo and Redo Subsystem](#undo-and-redo-subsystem)
  - [Selection Model](#selection-model)
  - [CLI and TUI Code Sharing](#cli-and-tui-code-sharing)
  - [Testing Approach](#testing-approach)
  - [Design Principles](#design-principles)
  - [Known Limitations](#known-limitations)

---

# Part 1: Features

## Views

Slate provides four primary views: Today, Upcoming, History, and Summary. You can navigate between them by pressing `Tab` (forward) or `Shift+Tab` (backward). Views can be conditionally hidden through configuration (`ui.show_upcoming`, `ui.show_history`, `ui.show_summary`).

### Today View

The Today view manages tasks scheduled for the current calendar day.

```text
  Slate    Today   Upcoming   History   Summary
  ────────────────────────────────────────────────────────────────────────

  Friday, 23 October    1 of 4 done  ███░░░░░░░    +3 upcoming
  ▸ Focus: Finalize Q4 Roadmap

  › ○ ▸ !! Finalize Q4 Roadmap #planning
    ○ ! Prepare slides 1/2 #work ≡
    └ ✓ First draft
    └ ○ Review comments
    ✓ ↻ !! Morning sync #team
    ○ Order coffee beans ≡

  a add  ·  space done  ·  t reschedule  ·  tab upcoming  ·  ? help  ·  q quit
```

Elements displayed in the Today view:
- **Title header**: Formatted current date, top-level task completion count (`1 of 4 done`), visual progress bar, and status badges (`+3 upcoming`, `1 overdue`, `N-day streak`). When terminal width is constrained, badges drop in order: overdue count, upcoming count, streak indicator.
- **Carry-over hint line**: Appears below the date header when previous days contain uncompleted tasks (`N unfinished from yesterday · c to carry over`).
- **Focus pin line**: Appears below the carry-over line when a focus task is set (`▸ Focus: <text>`).
- **Task row**: Selection marker (`›`), completion marker (`○` / `✓`), focus marker (`▸`), recurrence marker (`↻`), priority indicator (`!` / `!!`), task text, subtask completion count (`1/2`), tags (`#work`), and note presence indicator (`≡`).
- **Subtask row**: Indented subtask connector (`└`), completion marker, subtask text, and inline attributes.
- **Footer**: Dynamic context-sensitive keybinding hints.

### Upcoming View

The Upcoming view organizes future tasks by date.

```text
  Slate    Today   Upcoming   History   Summary
  ────────────────────────────────────────────────────────────────────────

  › Tomorrow           0/2│  Saturday, 24 October   •   0 of 2 done
    Mon 26 Oct         0/1│
                          │  › ○ Plan sprint backlog #work
                          │    ○ !! Doctor appointment

  enter/l tasks  ·  tab history  ·  ? help  ·  q quit
```

Layout:
- **Left pane (Dates)**: Chronological list of future calendar dates that contain at least one task, alongside the completion ratio (`done/total`) for each date.
- **Vertical divider rule**: Separates dates from task details.
- **Right pane (Tasks)**: Header showing full date and completion count, followed by the task list for the selected future date.
- Navigation: `h` or `Left` focuses the dates list. `l`, `Right`, or `Enter` focuses the tasks list.

### History View

The History view provides an archive of past calendar days.

```text
  Slate    Today   Upcoming   History   Summary
  ────────────────────────────────────────────────────────────────────────

  › Mon 26 Oct         0/1│  Monday, 26 October   •   0 of 1 done
    Tomorrow           0/2│
    Today              1/4│  › ○ ! Review security audit #infra
    Yesterday          1/1│

  enter/l tasks  ·  tab today  ·  ? help  ·  q quit
```

Layout:
- **Left pane (Dates)**: Reverse-chronological list of past dates that contain tasks. Relative labels (`Yesterday`) are used when applicable.
- **Right pane (Tasks)**: Tasks for the selected date. Completed tasks display their recorded completion timestamp (`done 14:32` in 24h format, or `done 2:32 pm` in 12h format).
- Navigation mirrors the Upcoming view: `h` / `Left` for dates, `l` / `Right` / `Enter` for tasks.

### Summary View

The Summary view displays completion aggregates and heatmaps. It supports two ranges toggled with `w`: **Week** and **Month**.

#### Week Range

```text
  Slate    Today   Upcoming   History   Summary
  ────────────────────────────────────────────────────────────────────────

    Week of Mon 19 Oct – Sun 25 Oct
    ────────────────────────────────────────────────────────────────────
    Mon 19 Oct    –
    Tue 20 Oct    –
    Wed 21 Oct    –
    Thu 22 Oct    ██████████   1/1
  › Fri 23 Oct    ███░░░░░░░   1/4
    Sat 24 Oct    2 scheduled
    Sun 25 Oct    –
    Week  2 of 7 done · 28%

  w week/month  ·  enter open  ·  a add today  ·  tab today  ·  ? help  ·
```

Each row corresponds to one day of the active 7-day week (respecting `week_start`). Completed days show a progress bar and completion fraction. Future days with pending tasks display `<N> scheduled`. Empty days display `–`. A summary line displays total week completions and percentage.

#### Month Range

```text
  Slate    Today   Upcoming   History   Summary
  ────────────────────────────────────────────────────────────────────────

    October 2026
    ────────────────────────────────────────────────────────────────────
    Week of 28 Sep         · ·  · ·          –
    Week of 5 Oct    · · · · ·  · ·          –
    Week of 12 Oct   · · · · ·  · ·          –
  › Week of 19 Oct   · · · █ ▒  2 ·      2/7   28%
    Week of 26 Oct   1 · · · ·  ·        0/1    0%
    Month  2 of 8 done · 25%

  w week/month  ·  enter open  ·  a add today  ·  tab today  ·  ? help  ·
```

The month range displays calendar week rows. Each day is represented by an activity cell with weekend spacing separating weekdays and weekends.
- Past completed days use density shades: `░` (1–25%), `▒` (26–50%), `▓` (51–75%), `█` (76–100%).
- Past days with no completions display `·`.
- Future days with scheduled tasks show a numeric task count.
- Pressing `Enter` on a week row drills into that week in the Week range. Pressing `Enter` on a day row in Week range opens that day in Today, Upcoming, or History.
- Pressing `[` and `]` navigates to the previous or next period. Pressing `g` jumps to the current period.

---

## Working with Tasks

- **Add task (`a`)**: Opens the inline prompt. Type text and optional quick-add tokens, then press `Enter` to commit or `Esc` to cancel.
- **Edit task (`e`)**: Opens the inline prompt pre-filled with the selected task text, tags, priority, and future date. Pressing `Enter` updates the task in-place.
- **Toggle completion (`Space` / `Enter`)**: Toggles the task between pending (`○`) and completed (`✓`). If tasks are marked, toggles all marked tasks.
- **Delete task (`d` / `Delete`)**: Removes the selected task. If the task has subtasks, all child tasks are deleted. If tasks are marked, deletes all marked tasks.
- **Clear completed (`D`)**: Removes all completed tasks on the current day in Today view or the active date in History view.
- **Reorder (`J` / `K`)**: Moves the selected task down or up by one row. Only active when sorting is set to `manual`. Moving a parent task moves its subtasks with it.
- **Move to Today (`m`)**: Moves a task from Upcoming or History into Today, setting its created date to today and resetting its completion state to unfinished.
- **Carry over (`c`)**: When unfinished tasks exist on the most recent previous active day, pressing `c` moves those unfinished top-level tasks and their subtasks to Today.
- **Overdue indicator (`!`)**: Historical tasks that remain unfinished display a warning-colored `!` indicator in the History list, and an overdue badge appears in the Today view header.

---

## Quick-Add Syntax

Slate parses task descriptions with a unified whitespace-delimited tokenizer shared between TUI prompts and the CLI `add` command.

| Token | Syntax | Examples | Meaning |
| :--- | :--- | :--- | :--- |
| **Date** | `@<date>` | `@today`, `@tomorrow`, `@fri`, `@2026-11-15`, `@11-15`, `@+3d`, `@+2w` | Schedules the task for that date. Unambiguous past dates are rejected. |
| **Recurrence** | `@<cadence>` | `@daily`, `@weekdays`, `@weekly` | Creates a repeating task template. |
| **Priority** | `!` or `!!` | `!`, `!!` | Sets priority to 1 (`!`, accent) or 2 (`!!`, warning). |
| **Tag** | `#<tag>` | `#work`, `#finance-2026`, `#dev_ops` | Labels task. First character must be alphabetic. Max 32 chars, max 5 tags. |

### Escape and Text Rules

- **Literal prefix (`\`)**: Prefixing a token with `\` treats it as literal text without the backslash. For example, `\@today` produces the plain text `@today`, and `\!` produces `!`.
- **Preserved words**: Tokens starting with `@` that do not match date or recurrence formats (such as `@alice` or `name@example.com`) are kept verbatim as task text.
- **Numeric tags**: Tokens like `#123` are kept as plain text because tags must begin with a letter.
- **Case insensitivity**: Weekdays, months, tags, and recurrence tokens are matched case-insensitively. Tags are stored in lowercase.

### Validation and Error Cases

1. **Empty text**: Entering only tokens without task text produces `task text cannot be empty`.
2. **Date in past**: Quick-add dates before today (such as `@2020-01-01` or `@yesterday`) produce `date '<date>' cannot be in the past`.
3. **Conflicting dates**: Specifying two distinct date tokens (for example, `@tomorrow @fri`) produces `conflicting dates: '<d1>' and '<d2>'`.
4. **Recurrence date conflict**:
   - Combining `@daily` with a specific date produces an error because `@daily` implies the start date.
   - Combining `@weekdays` with a weekend date produces an error.
   - Combining `@weekly` with a specific weekday (for example, `@fri @weekly`) is valid and anchors weekly recurrence to that weekday.

---

## Richer Tasks

### Priorities and Sorting

- Tasks have three priority levels: `0` (normal, unflagged), `1` (`!`, accent), and `2` (`!!`, warning).
- Pressing `p` cycles the selected task's priority: `0 -> 1 -> 2 -> 0`.
- Pressing `s` toggles view projection sorting between `manual` and `priority`:
  - `manual`: Tasks appear in saved list order. Reordering (`J` / `K`) is enabled.
  - `priority`: Tasks with priority 2 appear first, followed by priority 1, then priority 0. Reordering is disabled. Sorting is purely a display projection; it never mutates on-disk list order.

### Tags and Filtering

- Tasks can hold up to 5 lowercase alphanumeric tags (supports `-` and `_`).
- In-memory filtering (`f`) accepts `#tag` and priority filters (`!`, `!!`). For example, typing `#work !` filters the active view to tasks tagged `#work` with at least priority 1.
- Filters are local to each view (Today, Upcoming, History).
- An active filter is indicated in the header: `filter: #work !`.
- Pressing `Esc` clears selection marks on the first press and clears the active filter on the second press.

### Notes and the Detail Pane

- Pressing `n` opens the multi-line note editor for the selected task (up to 2000 characters).
  - Use `Alt+Enter` or `Ctrl+j` to insert a newline.
  - Press `Enter` to save or `Esc` to cancel.
- Tasks with notes show a note glyph (`≡` in Unicode, `=` in ASCII).
- Pressing `i` toggles the task detail pane, displaying full wrapped task text, scheduled dates, priorities, tags, subtask progress, and notes. The pane auto-hides when terminal height is below 20 rows.

### Subtasks

- Pressing `A` opens the subtask prompt under the selected task.
- Subtasks are nested exactly one level deep. Nesting a subtask under another subtask is rejected.
- Subtasks inherit their parent task's scheduled date. Quick-add date and recurrence tokens are prohibited on subtasks.
- Parent tasks display progress counts (`1/3`) indicating completed versus total subtasks.
- **Completion cascade rules**:
  - Marking a parent task done automatically marks all its child subtasks done.
  - Unmarking any subtask automatically re-opens its parent task if the parent was marked done.

### Focus Task

- Pressing `P` pins or unpins the selected task as the global focus task.
- The focus task is highlighted with a `▸` glyph and displayed prominently on Today:
  `▸ Focus: Finalize Q4 Roadmap`
- Only one focus task can exist across the entire application. Pinning a new task replaces the previous focus pin.
- When a focused task is completed or deleted, the focus pin automatically clears.

### Recurring Tasks

- Pressing `r` opens the recurrence prompt: `daily`, `weekdays`, `weekly`, or `off`.
- When scheduled, recurring tasks materialize a new instance on rollover or when Slate starts up.
- **No back-filling**: If Slate is not run for several days, it does not generate redundant tasks for past missed days. It materializes only the instance for today or the next active day.
- Recurring instances show the `↻` repeat glyph (`~` in ASCII).
- Deleting a recurring instance prompts whether to delete only that single instance or cancel the entire recurring template.

### Multi-Select and Bulk Actions

- Press `v` to toggle the selection mark on the current task. Marked rows display a diamond marker (`◆` in Unicode, `*` in ASCII).
- Press `V` to toggle marks on all visible tasks in the current view.
- When marks are present:
  - `Space` / `Enter` toggles completion for all marked tasks.
  - `d` / `Delete` deletes all marked tasks.
  - `m` moves all marked tasks to Today.
  - `t` reschedules all marked tasks to a target date.
- All bulk mutations are committed as a single snapshot on the undo stack.
- Pressing `Esc` clears all active marks.

### Undo and Redo

- Press `u` to undo the previous mutation.
- Press `Ctrl+r` to redo an undone mutation.
- Slate maintains an in-memory snapshot stack holding up to 50 operations.
- Any new mutation clears the redo stack.
- When an external change is detected and reloaded from disk, the undo and redo history is cleared to prevent reconciling conflicting disk states.

---

## Finding Things

### Search Syntax

Press `/` to open the full-screen search view across all dates (past, today, future).

Supported search tokens:
- **Plain text**: Matches case-insensitive substrings in task text and notes. Multiple words act as an `AND` query.
- **`#tag`**: Matches tasks containing the specified tag.
- **`!`**: Matches tasks with priority 1 or higher.
- **`!!`**: Matches tasks with priority 2.
- **`is:done`**: Matches completed tasks only.
- **`is:open`**: Matches unfinished tasks only.

Search features:
- Matching character ranges are highlighted in search results.
- If a match occurs only inside a note, a note excerpt is previewed below the result.
- Press `Up` / `Down` or `Ctrl+p` / `Ctrl+n` to navigate results.
- Press `Enter` to jump directly to the matched task in its native view (Today, Upcoming, or History).
- Press `Esc` to cancel search and restore prior view, selections, and filters.

### Jump to Date

Press `:` to open the jump prompt. Accepts:
- `today`, `yesterday`, `tomorrow`
- `YYYY-MM-DD` (e.g. `2026-11-15`)
- `MM-DD` (e.g. `11-15`, targets the current year)
- `+Nd` or `+Nw` (e.g. `+3d`, `+2w`)
- `-Nd` or `-Nw` (e.g. `-2d`, `-1w`)
- Weekday names (e.g. `fri`, `monday`), which resolve to the most recent past occurrence if jumping backward, or the next future occurrence.

If the requested date has no tasks, Slate navigates to the nearest date containing tasks.

### Streaks and Completion Rules

- Slate tracks daily task completion streaks. The streak count is computed from top-level tasks only; subtasks do not alter streak calculations.
- A day is considered complete according to `stats.streak_rule`:
  - `all` (default): All top-level tasks on that date must be completed (`done == total`, where `total > 0`).
  - `any`: At least one top-level task must be completed (`done > 0`).
- **Streak continuity**:
  - Consecutive complete days ending today (if today is complete) or ending yesterday (if today is incomplete).
  - An incomplete today does **not** break an active streak.
  - A past date with zero tasks or incomplete tasks **breaks** the streak.
- A streak hint appears on the Today header when `stats.show_streak = true` and the streak is 2 or more: `3-day streak`.

---

## Live Reload

Slate monitors `todos.json` for external modifications (such as edits made via CLI commands or another editor window).
- **Settling mechanism**: The internal change detector checks file metadata (`mtime`, file size, and Unix inode). To prevent reading half-written files, it requires two consecutive identical poll intervals before reloading.
- **Deferred prompts**: If you are actively typing in a mutating prompt (`add`, `edit`, `reschedule`, `note`), disk reload is deferred until you submit or press `Esc`. Your typed text is never overwritten.
- **Read-only prompts**: Read-only screens (search, help overlay) refresh immediately.
- **Undo history**: Reloading external data clears in-memory undo and redo stacks and displays `Reloaded · data changed on disk · undo history cleared`.

---

## Command Line

Slate includes a command-line interface sharing the same storage engine, locking mechanism, and quick-add parser as the TUI.

```text
Usage: slate [OPTIONS] [COMMAND]

Commands:
  add     Add one or more tasks
  done    Mark tasks done
  undone  Mark tasks unfinished
  rm      Delete tasks
  list    List tasks
  today   Summary for today (weekday header, progress, tasks)
  help    Print this message or the help of the given subcommand(s)

Options:
      --config <PATH>  Path to config file
  -h, --help           Print help
  -V, --version        Print version
```

### Addressing Tasks

Commands operating on existing tasks (`done`, `undone`, `rm`) address tasks by 1-based index or substring match:
- **Top-level index (`N`)**: e.g. `slate done 1` or `slate rm 2 3`.
- **Subtask index (`N.M`)**: e.g. `slate done 1.2`.
- **Substring match (`--match <TEXT>`)**: Matches case-insensitive substring against a single task. Fails with exit code 3 if no tasks match or if multiple tasks match.

### Date Parameters

Commands accepting `--date <DATE>` support:
- `today`
- `yesterday`
- `tomorrow`
- `YYYY-MM-DD` (e.g. `2026-11-15`)

### Non-Interactive and Piped Output

- Running `slate` without a subcommand when stdout is not a terminal exits with status 1: `slate: not a terminal (try "slate list")`.
- When stdout is piped or redirected, CLI commands automatically strip ANSI escape sequences and render ASCII glyphs (`[ ]`, `[x]`, `` ` ``).
- Broken pipe errors (such as `slate list | head -n 1`) are handled gracefully without panics.

### Subcommands

#### `add`

```text
Usage: slate add [OPTIONS] <TEXT>...

Arguments:
  <TEXT>...  Task text to add (or - to read line-by-line from stdin)

Options:
      --config <PATH>  Path to config file
      --date <DATE>    Target date (today, yesterday, tomorrow, or YYYY-MM-DD)
      --parent <N>     Parent task index to add as a subtask
  -q, --quiet          Do not print confirmation
```

Examples:
```bash
slate add "Buy milk"
slate add "Submit quarterly report @fri #work !!"
slate add --parent 1 "Draft introduction"
cat backlog.txt | slate add -
```

#### `done`

```text
Usage: slate done [OPTIONS] [N]...

Arguments:
  [N]...  1-based task indices (e.g. 1, 1.1)

Options:
      --config <PATH>  Path to config file
      --date <DATE>    Target date (today, yesterday, tomorrow, or YYYY-MM-DD)
      --match <TEXT>   Case-insensitive substring to match a single task
```

Examples:
```bash
slate done 1
slate done 1 2
slate done --match "dentist"
slate done --date yesterday 1
```

#### `undone`

Marks completed tasks unfinished. Flags and arguments mirror `done`.

Examples:
```bash
slate undone 1
slate undone --match "milk"
```

#### `rm`

Deletes tasks. Flags and arguments mirror `done`. Deleting a parent task automatically deletes all of its subtasks.

Examples:
```bash
slate rm 1
slate rm --match "temporary"
```

#### `list`

```text
Usage: slate list [OPTIONS]

Options:
      --config <PATH>       Path to config file
      --date <DATE>         Target date (today, yesterday, tomorrow, or YYYY-MM-DD)
      --all                 Show tasks across all days (newest first)
      --pending             Show unfinished tasks only
      --limit <N>           Limit number of tasks displayed
      --tag <TAG>           Filter tasks by tag (case-insensitive)
      --min-priority <1|2>  Filter tasks by minimum priority (1 for !, 2 for !!)
      --upcoming            Show tasks on future days
```

`--all`, `--upcoming`, and `--date` are mutually exclusive.

Examples:
```bash
slate list
slate list --pending
slate list --all --limit 10
slate list --tag work --min-priority 1
slate list --upcoming
```

#### `today`

Prints a summary for today formatted with date header, progress count, visual bar, and numbered tasks.

```bash
$ slate today
Saturday, 31 October
1 of 2 done  █████░░░░░

   1  ✓  Buy milk           done 21:05
   2  ○  Silent task
```

### Exit Codes

| Code | Meaning | Triggers |
| :---: | :--- | :--- |
| `0` | Success | Command executed successfully. |
| `1` | Runtime error | Corrupt storage file, lock acquisition timeout, I/O failure, or running bare `slate` without a terminal. |
| `2` | Usage error | Invalid date syntax, conflicting options (`--all` with `--date`), empty task text, or empty `--match`. |
| `3` | Target not found | Task index out of range, index 0, `--match` found zero hits, or `--match` found multiple ambiguous hits. |

---

## Customization

Configuration is loaded from standard OS paths or the `SLATE_CONFIG` environment variable.
- Linux: `~/.config/slate/config.toml`
- macOS: `~/Library/Application Support/slate/config.toml`
- Windows: `%APPDATA%\slate\config.toml`

Precedence order (lowest to highest):
`built-in defaults < theme preset < config.toml < environment variables`

### Configuration Options

| Section | Option | Type | Default | Valid Values / Clamping |
| :--- | :--- | :--- | :--- | :--- |
| *root* | `theme` | string | `"teal"` | `"teal"`, `"orange"`, `"lavender"`, `"mono"` |
| *root* | `accent` | string | `None` | 6-digit hex (`"#4ebaaf"`) or ANSI color name |
| *root* | `date_format` | string | `"%A, %d %B"` | Valid strftime format string |
| *root* | `short_date_format` | string | `"%a %d %b"` | Valid strftime format string |
| *root* | `time_format` | string | `"24h"` | `"24h"`, `"12h"` |
| *root* | `week_start` | string | `"monday"` | `"monday"`, `"sunday"`, `"saturday"` |
| `[ui]` | `symbols` | string | `"unicode"` | `"unicode"`, `"ascii"` |
| `[ui]` | `show_history` | bool | `true` | `true`, `false` |
| `[ui]` | `show_upcoming` | bool | `true` | `true`, `false` |
| `[ui]` | `show_summary` | bool | `true` | `true`, `false` |
| `[ui]` | `detail_pane` | bool | `false` | `true`, `false` |
| `[ui]` | `mouse` | bool | `false` | `true`, `false` |
| `[progress]` | `style` | string | `"block"` | `"block"`, `"line"`, `"dots"`, `"ascii"`, `"none"` |
| `[progress]` | `width` | integer | `10` | Clamped to `4..=40` |
| `[progress]` | `show_count` | bool | `true` | `true`, `false` |
| `[tasks]` | `sort` | string | `"manual"` | `"manual"`, `"priority"` |
| `[tasks]` | `show_tags` | bool | `true` | `true`, `false` |
| `[summary]` | `default_range` | string | `"week"` | `"week"`, `"month"` |
| `[stats]` | `show_streak` | bool | `true` | `true`, `false` |
| `[stats]` | `streak_rule` | string | `"all"` | `"all"`, `"any"` |
| `[storage]` | `live_reload` | bool | `true` | `true`, `false` |
| `[storage]` | `reload_interval_ms` | integer | `500` | Clamped to `250..=5000` |
| `[keys]` | `mode` | string | `"default"` | `"default"`, `"arrows"` |

### Themes and Colors

Preset palettes:
- `teal` (default): Soft teal accent (`rgb(78, 186, 175)`), dark gray dividers, gray metadata, light red errors.
- `orange`: Warm orange accent (`rgb(228, 140, 75)`).
- `lavender`: Soft purple accent (`rgb(180, 140, 220)`).
- `mono`: Disables foreground hue. Uses terminal default foreground with bold, dim, and underline text modifiers. Overdue indicators and errors remain distinguishable.

Overriding accent color:
```toml
accent = "#4ebaaf" # Or "cyan", "lightblue", etc.
```

### Monochrome and ASCII Modes

- **`NO_COLOR`**: If the `NO_COLOR` environment variable is set (to any non-empty value), Slate automatically enforces `preset_mono()` regardless of the `theme` setting.
- **ASCII mode**: Setting `[ui] symbols = "ascii"` or the environment variable `SLATE_ASCII=1` replaces all Unicode glyphs with pure ASCII equivalents:
  - `○` -> `[ ]`
  - `✓` -> `[x]`
  - `▸` -> `>`
  - `└` -> `` ` ``
  - `≡` -> `=`
  - `↻` -> `~`
  - `█` / `░` -> `#` / `-`

### Progress Bar Styles

Configured via `[progress] style`:
- `block`: Uses Unicode fractional blocks `█░` (default).
- `line`: Uses horizontal line characters `━─`.
- `dots`: Uses dot characters `●○`.
- `ascii`: Uses ASCII hash and hyphen characters `#-`. Enforced when symbols mode is `ascii`.
- `none`: Hides the progress bar entirely.

### Keymap Customization

In `[keys]`, action names can be assigned a single key string or list of key strings. Overriding an action completely replaces its default bindings:

```toml
[keys]
mode = "default" # "default" or "arrows"
add = ["a", "insert"]
toggle = ["space", "enter"]
quit = ["q"]
```

Key string format:
- Single characters: `"a"`, `"e"`, `"?"`, `"/"`
- Special keys: `"enter"`, `"esc"`, `"space"`, `"tab"`, `"backtab"`, `"up"`, `"down"`, `"left"`, `"right"`, `"home"`, `"end"`, `"pageup"`, `"pagedown"`, `"delete"`, `"backspace"`
- Modifiers: `"ctrl+"`, `"alt+"`, `"shift+"` (e.g. `"ctrl+r"`, `"alt+enter"`)
- Safety: `Ctrl+c` is hardcoded to quit and cannot be rebound.
- Conflict resolution: If the same key is assigned to multiple actions, Slate keeps the first registered binding, discards the duplicate, and emits a startup warning.

### Mouse Support

Mouse capture is disabled by default so native terminal text selection remains uninterrupted. Setting `[ui] mouse = true` enables:
- Clicking header tabs to switch views.
- Clicking rows to move selection.
- Clicking checkboxes to toggle completion.
- Mouse wheel scrolling up and down through lists.

---

## Full Keyboard Reference

All shortcuts generated directly from defaults in `src/keymap.rs`.

### Navigation and General Actions

| Default Binding | Arrows Mode | Action Name | Function |
| :--- | :--- | :--- | :--- |
| `q` | `q` | `quit` | Exit application |
| `Ctrl+c` | `Ctrl+c` | *(hardcoded)* | Force quit |
| `Tab`, `Shift+Tab` | `Tab`, `Shift+Tab` | `switch_view` | Cycle views forward / backward |
| `k`, `Up` | `Up` | `move_selection_up` | Move cursor up |
| `j`, `Down` | `Down` | `move_selection_down` | Move cursor down |
| `g`, `Home` | `Home` | `jump_top` | Jump to first item in list |
| `G`, `End` | `End` | `jump_bottom` | Jump to last item in list |
| `Ctrl+u`, `PageUp` | `PageUp` | `half_page_up` | Scroll up half page |
| `Ctrl+d`, `PageDown` | `PageDown` | `half_page_down` | Scroll down half page |
| `?` | `?` | `help` | Toggle help overlay |
| `/` | `/` | `search` | Open global search screen |
| `:` | `:` | `jump_to_date` | Open jump-to-date prompt |

### Task Operations

| Default Binding | Arrows Mode | Action Name | Function |
| :--- | :--- | :--- | :--- |
| `a` | `a` | `add` | Add new task |
| `A` | `A` | `add_subtask` | Add subtask under selected task |
| `e` | `e` | `edit` | Edit selected task |
| `Space`, `Enter` | `Space`, `Enter` | `toggle` | Toggle task completion |
| `d`, `Delete` | `d`, `Delete` | `delete` | Delete selected task |
| `D` | `D` | `clear_completed` | Clear all completed tasks on date |
| `u` | `u` | `undo` | Undo last operation |
| `Ctrl+r` | `Ctrl+r` | `redo` | Redo undone operation |
| `c` | `c` | `carry_over` | Carry over previous unfinished tasks |
| `m` | `m` | `move_to_today` | Move selected task to Today |
| `t` | `t` | `reschedule` | Reschedule task to new date |
| `p` | `p` | `cycle_priority` | Cycle priority (none, `!`, `!!`) |
| `s` | `s` | `cycle_sort` | Cycle sort mode (`manual` / `priority`) |
| `f` | `f` | `filter` | Filter tasks by tag or priority |
| `i` | `i` | `toggle_detail` | Toggle task detail pane |
| `n` | `n` | `edit_note` | Open note editor for task |
| `P` | `P` | `toggle_focus` | Toggle focus task pin |
| `r` | `r` | `recurrence` | Set task recurrence rule |
| `v` | `v` | `toggle_mark` | Toggle selection mark on task |
| `V` | `V` | `toggle_mark_all` | Toggle marks on all visible tasks |
| `K` | `Shift+Up` | `reorder_up` | Move task up (manual sort only) |
| `J` | `Shift+Down` | `reorder_down` | Move task down (manual sort only) |

### History and Upcoming Split Navigation

| Default Binding | Arrows Mode | Action Name | Function |
| :--- | :--- | :--- | :--- |
| `h`, `Left` | `Left` | `focus_dates` | Move focus to left dates column |
| `l`, `Right` | `Right` | `focus_tasks` | Move focus to right tasks column |

### Summary View

| Default Binding | Arrows Mode | Action Name | Function |
| :--- | :--- | :--- | :--- |
| `w` | `w` | `toggle_summary_range` | Toggle between Week and Month range |
| `[` | `[` | `prev_period` | Navigate to previous week/month |
| `]` | `]` | `next_period` | Navigate to next week/month |

---

# Part 2: How It Works

## Architecture Overview

Slate is built as a single Rust binary that links against `ratatui` for terminal UI rendering, `crossterm` for terminal control, `serde_json` for file serialization, and `clap` for command-line argument dispatch.

### Source Layout

- `src/main.rs`: Entry point. Parses CLI options, initializes the terminal, runs the main event loop, and installs a panic hook to safely restore terminal state.
- `src/lib.rs`: Library root exposing public modules and path resolution.
- `src/app.rs`: State machine, keyboard and mouse dispatch, selection tracking, mutation lifecycle, and undo/redo snapshot stacks.
- `src/model.rs`: Core domain types: `Todo`, `Recurring`, `RecurrenceRule`, `View`, `InputMode`, `SortMode`, and `Filter`.
- `src/storage/mod.rs`: Load, atomic save, in-memory repairs, and fingerprint detection.
- `src/storage/lock.rs`: Sidecar file lock acquisition with polling timeout.
- `src/storage/backup.rs`: Rolling snapshot creation, timestamp deduplication, and retention pruning.
- `src/storage/migrate.rs`: Schema detection and upgrade pipeline from v0 and v1 to v2.
- `src/store.rs`: Pure domain logic: task reordering, completion cascades, recurring task materialization, and carry-over.
- `src/parser.rs`: Quick-add tokenizer, date expression parsing, and edit prompt re-serialization.
- `src/dates.rs`: Week and month calendar math, period bounds, and weekday offsets.
- `src/search.rs`: Unicode-safe search tokenization, character-range calculation, and hit ranking.
- `src/stats.rs`: Streak computation, completion aggregates, and DaySummary caching.
- `src/watch.rs`: Change detection state machine, settling timers, and poll timeout calculations.
- `src/config.rs`: TOML config loader, validation, clamping rules, and environment lookups.
- `src/keymap.rs`: Key chord parsing, action resolution, conflict detection, and arrow mode definitions.
- `src/theme.rs`: Color presets, ANSI palette mappings, and `NO_COLOR` monochrome adaptation.
- `src/ui.rs`: Ratatui view layout, symbol mappings, progress bar rendering, and terminal size guard.
- `src/cli.rs`: Clap command definitions, options, and documentation examples.
- `src/commands.rs`: Execution handlers for CLI subcommands (`add`, `done`, `undone`, `rm`, `list`, `today`).
- `src/output.rs`: Plain text formatting, column alignments, and `SafeWriter` broken-pipe handling.

### Data Flow Diagram

```text
 Terminal Event (Key / Mouse / Resize)
                 │
                 ▼
         keymap::Keymap
                 │
           Action enum
                 │
                 ▼
              app::App
                 │
   ┌─────────────┴─────────────┐
   │ Check for external reload │
   │ Push undo snapshot        │
   │ Mutate store::*           │
   │ Invalidate stats cache    │
   │ Save storage::* (atomic)  │
   └─────────────┬─────────────┘
                 │
                 ▼
              ui::render
                 │
                 ▼
   ratatui::Terminal::draw
```

---

## The Event Loop

The TUI event loop in `src/main.rs` runs synchronously on the main thread:
1. **Dynamic Timeout Calculation**:
   The timeout passed to `event::poll(timeout)` is calculated via `watch::next_timeout`:
   - If `storage.live_reload` is enabled, the timeout is `min(time_until_midnight, reload_interval_ms)`, floored to at least 1 millisecond.
   - If live reload is disabled, the timeout sleeps until midnight (capped at 60 seconds).
2. **Event Polling**:
   - `Event::Key`: Dispatched to `app.handle_key(key)`. Returns true if the user requested quit (`Action::Quit`).
   - `Event::Mouse`: Dispatched to `app.handle_mouse(event)` if mouse capture is active.
   - `Event::Resize`: Updates terminal dimensions in `app.on_resize(w, h)` and requests a redraw.
3. **Change Detection**:
   `app.check_for_external_changes()` stats `todos.json`. If a new fingerprint has settled over two poll intervals and no mutating input prompt is open, it reloads external data.
4. **Midnight Rollover**:
   `app.tick(today)` checks if the local date has changed. If midnight has passed, `app.today` advances, recurring tasks for the new day materialize, and the screen redraws.
5. **Redraw Guard**:
   `terminal.draw()` is invoked only when `needs_redraw` is true.

---

## State and Data Model

### Data Shapes

`Todo` represents an individual item on a specific calendar date:
- `id`: Unique 64-bit unsigned integer.
- `text`: Plain text string description.
- `created_date`: `NaiveDate` representing the date the task belongs to.
- `done`: Boolean completion flag.
- `done_at`: Optional timestamp recorded when marked completed (`DateTime<Local>`).
- `priority`: 8-bit unsigned integer (`0`, `1`, or `2`).
- `tags`: Vector of lowercase string labels.
- `note`: Optional multi-line text string.
- `parent`: Optional 64-bit integer referencing parent task ID (for subtasks).
- `recurring_id`: Optional 64-bit integer referencing source recurring template.

`Recurring` represents a recurring rule template:
- `id`: Unique 64-bit template ID.
- `text`, `tags`, `priority`: Inherited attributes for newly generated tasks.
- `rule`: `RecurrenceRule` (`daily`, `weekdays`, `weekly`).
- `anchor`: Start date.
- `last_materialized`: Date of the most recently created instance.

### On-Disk Schema (Version 2)

```json
{
  "version": 2,
  "next_id": 11,
  "todos": [
    {
      "id": 1,
      "text": "Finalize Q4 Roadmap",
      "created_date": "2026-10-23",
      "done": false,
      "done_at": null,
      "priority": 2,
      "tags": ["planning"],
      "note": null,
      "parent": null,
      "recurring_id": null
    },
    {
      "id": 2,
      "text": "First draft",
      "created_date": "2026-10-23",
      "done": true,
      "done_at": "2026-10-23T11:45:00+05:30",
      "priority": 0,
      "tags": [],
      "note": null,
      "parent": 1,
      "recurring_id": null
    }
  ],
  "recurring": [
    {
      "id": 101,
      "text": "Morning sync",
      "tags": ["team"],
      "priority": 2,
      "rule": "daily",
      "anchor": "2026-10-23",
      "last_materialized": "2026-10-23"
    }
  ],
  "focus": 1
}
```

---

## Storage and Safety

### Atomic File Replacement

To guarantee crash resilience and prevent partial writes:
1. Slate serializes data to pretty-printed JSON in memory.
2. It writes to a temporary file in the same directory: `.<file_name>.tmp.<pid>.<timestamp_nanos>`.
3. It calls `sync_all()` on the temporary file handle, forcing data from OS buffers to physical disk.
4. It renames the temporary file over the destination path (`rename_with_retries`), retrying up to 5 times on Windows to account for transient virus scanner locks.
5. On Unix systems, it opens the parent directory and calls `sync_all()` to ensure the directory entry update is committed.

### Advisory Locking

All write operations (both TUI mutations and CLI commands) lock `<data_path>.lock` using `fs2::FileExt::try_lock_exclusive`.
- If another process holds the lock, Slate polls every 25 milliseconds until the lock is acquired.
- If the lock cannot be acquired within 3 seconds, the operation aborts with an error, preventing deadlocks or data corruption.

### Rolling Backups

Snapshots are written to a `backups/` subdirectory beside `todos.json`.
- File naming: `todos-YYYYMMDD-HHMMSS.json`.
- Backup triggers:
  - First save of a session.
  - Periodic save when 10 minutes (600 seconds) have elapsed since the prior backup.
  - Prior to writing when an external disk change was detected.
  - Prior to applying a schema migration.
- Deduplication: Slate compares file bytes; if identical to the newest existing backup, snapshot creation is skipped.
- Pruning: Slate sorts snapshot filenames and deletes older files, keeping the 10 newest backups.

### Schema Migrations

Slate automatically migrates legacy schema versions:
- **v0**: Top-level JSON array of tasks (`[...]`). Migrates to v1, creating `todos.v0.bak`.
- **v1**: Object with `"version": 1` and `"todos": [...]`. Migrates to v2 by allocating explicit IDs, subtask pointers, priority, notes, recurring templates, and creating `todos.v1.bak`.
- **v2**: Current active schema.

### Error and Corruption Handling

- **Newer schema versions**: If `version > 2`, Slate refuses to load or overwrite the file, cleanly reporting:
  `todos.json was written by a newer Slate (version X, this build supports 2). Upgrade Slate.`
- **Malformed JSON**: If parsing fails, Slate points to the exact line and column, inspects `backups/`, and displays the path of the newest readable backup.
- **In-memory repairs**: When loading, Slate corrects minor structural issues in memory:
  - Deduplicates conflicting IDs.
  - Promotes orphaned subtasks whose parents do not exist to top-level tasks.
  - Re-anchors subtasks on different dates to top-level tasks.
  - Flattens nested subtasks beyond 1 level.
  - Clears missing focus task pointers.
  - Re-opens completed parents if any child subtask is unfinished.

---

## Rendering Engine

- **Unboxed layout**: Avoids heavy double-line ASCII boxes. Uses 1-2 columns of breathable horizontal padding, thin divider rules (`h_rule`), and vertical whitespace.
- **Theme and symbols separation**:
  - `Theme` manages colors and styles.
  - `Symbols` manages glyph sets (`Unicode` vs `Ascii`).
- **Minimum terminal size**: Enforces a minimum resolution of 40 columns by 8 rows (`MIN_WIDTH = 40`, `MIN_HEIGHT = 8`). When below this threshold, normal view drawing is suspended and a centered message is rendered: `Terminal too small · need 40×8, have W×H`.

---

## Keymap and Action Resolution

- Crossterm `KeyEvent` values are converted into `KeyChord` structs (`code: KeyCode`, `modifiers: KeyModifiers`).
- `Keymap::resolve` scans active bindings to produce an `Action`.
- `Ctrl+c` is evaluated before any custom keybinding and always triggers `Action::Quit`.

---

## Undo and Redo Subsystem

- Mutations push a full `Snapshot` (`todos`, `recurring`, `focus`) to `undo_stack`.
- The stack holds up to 50 operations. Pushing a 51st operation drops the oldest snapshot.
- Redo snapshots are stored in `redo_stack`. Any new user mutation invalidates the redo stack.
- External disk changes clear both undo and redo stacks because in-memory diffs cannot be safely projected over externally changed state.

---

## Selection Model

Selections are tracked by persistent 64-bit task IDs and `NaiveDate` calendar dates rather than raw list indices:
- If a list is reordered or filtered, the cursor remains pinned to the same task ID.
- If the selected task is deleted externally, the cursor lands on its nearest neighbor.
- In History and Upcoming views, if the selected date vanishes, selection falls back to the nearest remaining calendar date.

---

## CLI and TUI Code Sharing

The CLI commands and TUI interface share the same core modules:
- File persistence: `try_load_modify_save`, `load_todos`, and `save_storage_data_unlocked`.
- Text parsing: `parse_quick_add` and `parse_date_expression`.
- Recurring tasks: `store::materialize_recurring`.
- Locking: Both environments acquire `todos.json.lock` before touching disk, allowing safe interleaved operations.

---

## Testing Approach

The codebase maintains 243 automated tests across three tiers:
1. **Unit tests**: Pure functions in `parser`, `dates`, `stats`, `search`, and `store`.
2. **Terminal rendering tests**: Use Ratatui's in-memory `TestBackend` to verify screen layouts, color assignments, ASCII fallback, and terminal resize boundaries without opening a physical terminal.
3. **Integration tests**: In `tests/cli_tests.rs`, run CLI commands against isolated temporary directories (`tempfile::tempdir()`).

> [!IMPORTANT]
> Tests never touch the user's real data directory (`~/.local/share/slate`). Temporary paths are injected for all disk operations.

Run the test suite:
```bash
cargo test
```

---

## Design Principles

1. **Quiet, Breathable Interface**: Prioritize typography, whitespace, and subtle accents over dense borders and saturated colors.
2. **Never Lose Data**: Always write through temporary files, fsync before rename, maintain rolling backups, lock against concurrent writes, and refuse to overwrite unreadable files.
3. **Deterministic Logic**: Keep domain rules (sorting, streaks, recurring materialization) pure and covered by automated tests.
4. **Resilient Input Handling**: Never panic on malformed user input, irregular dates, or corrupt configuration files. Preserve unknown tokens as plain text.

---

## Known Limitations

1. **Single-level subtasks**: Subtasks cannot be nested under another subtask.
2. **Top-level counting**: Progress bars and streak statistics calculate completions based on top-level tasks only.
3. **No future recurring projection**: Recurring tasks are materialized only when their date arrives; future instances are not previewed in the Upcoming view.
4. **No CRDT / three-way merge**: External file modifications are handled via whole-file reload; concurrent edits to different tasks in two separate instances will overwrite on save (last writer wins).
5. **In-memory retention on deletion**: Deleting `todos.json` while Slate is running does not clear the active list; Slate recreates the file on the next save.

