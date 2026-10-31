# Slate

[![CI](https://github.com/LakshThakre/Slate/actions/workflows/ci.yml/badge.svg)](https://github.com/LakshThakre/Slate/actions/workflows/ci.yml)

> A minimal, calm, and distraction-free terminal todo app built in Rust.

Slate is designed in the spirit of modern, quiet developer tools like Claude Code and Codex CLI. It discards heavy boxed borders in favor of generous whitespace, subtle padding, thin divider rules, and a restrained color palette that respects your terminal's natural theme.

---

## Features

- **Quiet, Minimal Interface**: Clean layout without heavy box borders, using subtle accents, breathable spacing, and dim metadata that appears only when present.
- **Cross-Platform**: Works smoothly across Linux, macOS, and Windows.
- **Runtime Theming & Presets**: Choose between `teal` (default), `orange`, `lavender`, or pure `mono` presets, or specify any custom hex (`#4ebaaf`) or ANSI accent color. Automatic 256-color fallback on terminals without 24-bit truecolor.
- **`NO_COLOR` & ASCII Compatibility**: Honors `NO_COLOR` per no-color.org (pure monochrome with text modifiers) and supports pure ASCII symbols fallback (`SLATE_ASCII=1` or `[ui] symbols = "ascii"`). Priorities, overdue, pinned, and marked rows remain distinguishable without color.
- **Rich Task Attributes without Clutter**:
  - **Priorities (`p`)**: Subtle priority flags (`!` accent, `!!` warn) that never distort row alignment.
  - **Tags & In-Memory Filter (`f`)**: Case-insensitive `#tag` labels with live filtering. Hidden on narrow terminals (< 60 cols).
  - **Scheduling & Rescheduling (`t`)**: Schedule tasks for future dates with `@date` expressions.
  - **Recurrence (`r`)**: Recurring templates for `@daily`, `@weekdays`, and `@weekly` cadences with safe non-backfilling materialization.
  - **Notes & Detail Pane (`n`, `i`)**: Multi-line note buffer (up to 2000 chars) and collapsible task detail pane with auto-hide below 20 terminal rows.
  - **Subtasks (`A`)**: One-level nested subtasks with parent progress indicators (`2/4`) and completion cascade invariants.
  - **Focus Task (`P`)**: Pin a single high-priority focus task globally displayed at the top of Today (`▸ Focus: ...`).
  - **Multi-Select & Bulk Actions (`v`, `V`)**: Mark multiple tasks to toggle, delete, move to today, or reschedule in a single undo step.
  - **Dynamic Sorting (`s`)**: View projection cycling between `manual` and `priority` (high first) without altering stored order.
- **Upcoming View**: Dedicated view listing future days with scheduled tasks in chronological order.
- **Today & History Views**:
  - **Today View**: See today's tasks with a friendly weekday header, block progress bar, and focus task pin.
  - **History View**: Browse past days by date with task counts and completion timestamps (`done 14:32`).
- **Unified Quick-Add Syntax**: Natural whitespace-delimited tokens (`@tomorrow`, `!!`, `#work`, `@weekly`) shared seamlessly between TUI and CLI.
- **Task Text Editing (`e`)**: Edit tasks with pre-serialized tokens in the inline prompt.
- **Full Undo / Redo (`u`, `Ctrl+r`)**: In-memory snapshot stack supporting all mutations including bulk actions as single undo steps.
- **Carry Over Unfinished Tasks (`c`)**: Hint line on startup or rollover showing unfinished tasks with one-key carry-over into Today.
- **Task Reordering (`J` / `K`)**: Move tasks or parent blocks within the list (active in manual sort mode).
- **Clear Completed (`D` / `Shift+d`)**: Clear all completed tasks for today or the selected historical date in a single action.
- **Jump Navigation (`g`, `G`, `Ctrl+d`, `Ctrl+u`)**: Quick jumps to the top, bottom, or half-page scroll in any focused list.
- **Help Overlay (`?`)**: Minimal modal overlay displaying all grouped keybindings from the active keymap.
- **Crash-Proof Hardened Storage (Schema v2)**: Saves automatically with atomic rename, rolling timestamped backups, and exclusive sidecar locking (`todos.json.lock`).
- **Stepwise Migrations & In-Memory Repairs**: Automatic migration from legacy v0 and v1 formats, creating a safety backup (`todos.v1.bak`). Automatic memory repair for orphan subtasks, invalid depths, or dangling focus IDs.
- **Midnight Daily Rollover**: Automatically advances `today` at local midnight with zero idle CPU spin.

---

## Visual Preview

### Today View with Metadata
```text
  Slate    Today   Upcoming   History
  ──────────────────────────────────────────────────────────────────────────

  ▸ Focus: Finalize Q4 Roadmap
  Friday, 23 October    2 of 4 done  █████░░░░░    +2 upcoming

  ›   ○  ! Prepare slides 1/2 #work
      └  ✓ First draft
      └  ○ Review comments
    ✓ ↻  !! Morning sync #team
    ○    Order coffee beans =

  a add  ·  A subtask  ·  p prio  ·  v mark  ·  ? help  ·  q quit
```

### Upcoming View
```text
  Slate   Today    Upcoming   History
  ──────────────────────────────────────────────────────────────────────────

  › Tomorrow        2 │  Saturday, 24 October
    Mon 26 Oct      1 │
    Fri 30 Oct      3 │  › ○ Plan sprint backlog #work
                      │    ○ !! Doctor appointment

  h dates  ·  t reschedule  ·  tab today  ·  ? help  ·  q quit
```

### Task Detail Pane (`i`)
```text
  Slate    Today   Upcoming   History
  ──────────────────────────────────────────────────────────────────────────

  ›   ○  ! Prepare slides 1/2 #work
      └  ✓ First draft
      └  ○ Review comments

  ──────────────────────────────────────────────────────────────────────────
  Prepare slides
  Scheduled: Fri, 23 Oct 2026   Priority: !   Tags: #work   Subtasks: 1/2
  Note: Include the updated benchmark graphs from the release build.
```

---

## Setup & Installation

Slate works across **Linux**, **macOS**, and **Windows**.

### Prerequisites
- **Rust toolchain** (version 1.85 or newer).
- A terminal with **Unicode (UTF-8)** support (e.g. Windows Terminal, Alacritty, Kitty, WezTerm, iTerm2, macOS Terminal, Ghostty, or VS Code integrated terminal).

---

### Step 1: Install Rust

If you do not have Rust installed yet, choose your operating system:

#### Linux
- **Arch Linux / Manjaro**:
  ```bash
  sudo pacman -S --needed rust
  ```
- **Ubuntu / Debian / Linux Mint**:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Fedora / RHEL**:
  ```bash
  sudo dnf install rust cargo
  ```

#### macOS
- **Via Homebrew**:
  ```bash
  brew install rust
  ```
- **Or via official rustup script**:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

#### Windows
- **Via winget (PowerShell / Command Prompt)**:
  ```powershell
  winget install Rustlang.Rustup
  ```
- **Or manual installer**:
  Download and run `rustup-init.exe` from [rustup.rs](https://rustup.rs).
  *(When prompted during installation, choose the default option and ensure the C++ Build Tools or MSVC toolchain is installed).*

---

### Step 2: Build and Install Slate

Clone or download this repository, then navigate into the project folder:

```bash
git clone https://github.com/LakshThakre/Slate.git
cd Slate
```

Install the binary directly using `cargo`:

```bash
cargo install --path .
```

`cargo install` compiles an optimized release binary and places `slate` (or `slate.exe` on Windows) directly into your user Cargo bin folder.

---

### Step 3: Ensure Cargo Bin is on your PATH

#### Linux & macOS
If `slate` is not found after installation, append Cargo's bin directory to your shell configuration (`~/.bashrc`, `~/.zshrc`, or `~/.config/fish/config.fish`):

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Then reload your shell:
```bash
source ~/.bashrc  # or source ~/.zshrc
```

#### Windows
`rustup` typically configures `%USERPROFILE%\.cargo\bin` in your User Environment variables automatically. If the command is not recognized, restart your PowerShell or Windows Terminal window.

You can verify the path in PowerShell with:
```powershell
$env:Path += ";$env:USERPROFILE\.cargo\bin"
```

---

### Step 4: Run Slate

From any terminal window, run:

```bash
slate
```

---

## Keyboard Shortcuts

> **Note:** The tables below show the **default** keybindings. Keybindings can be customized per action or switched to arrow-only navigation mode (`mode = "arrows"`) in [`config.toml`](#configuration). Footer hints and the help overlay (`?`) update dynamically to reflect your active keymap.

### Global Navigation
| Key | Action |
| :--- | :--- |
| `Tab` / `Shift+Tab` | Cycle between **Today**, **Upcoming**, and **History** views |
| `?` | Toggle help overlay (`Esc`, `?`, or `q` to close) |
| `u` | Undo last action (bulk actions are a single step) |
| `Ctrl+r` | Redo undone action |
| `s` | Cycle view sort order: `manual` ↔ `priority` |
| `f` | Filter tasks by `#tag` and/or `!` priority (empty input clears) |
| `i` | Toggle task detail pane (auto-hides when terminal height < 20) |
| `P` | Toggle global focus task pin (`▸ Focus: ...`) |
| `v` | Toggle selection mark on current task |
| `V` | Toggle mark all tasks in the current view |
| `Esc` | Clear selection marks (first press) / clear active filter (second press) |
| `g` | Jump to first item in focused list |
| `G` | Jump to last item in focused list |
| `Ctrl+d` | Move half a page down |
| `Ctrl+u` | Move half a page up |
| `q` | Quit Slate (closes overlays/prompts first if open) |

### Task Operations (Today & Upcoming Views)
| Key | Action |
| :--- | :--- |
| `a` | Add new task (opens inline quick-add prompt) |
| `A` | Add subtask under selected task (inherits parent date) |
| `e` | Edit selected task text (pre-populates metadata tokens) |
| `Space` / `Enter` | Toggle completion (`○` ↔ `✓`); toggles all marked if marks exist |
| `p` | Cycle priority: none → `!` (accent) → `!!` (warn) → none |
| `n` | Open multi-line note editor for selected task |
| `t` | Reschedule task (or marked tasks) to a new date expression |
| `r` | Set recurrence template (`daily`, `weekdays`, `weekly`, `off`) |
| `m` | Move task (or marked tasks) to Today |
| `d` / `Delete` | Delete selected task (cascades to subtasks; deletes marked if marks exist) |
| `D` / `Shift+d` | Clear all completed tasks for the current day |
| `J` / `Shift+j` | Move task down one position (manual sort only) |
| `K` / `Shift+k` | Move task up one position (manual sort only) |
| `c` | Carry over unfinished tasks from previous day |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |

### History View
| Column | Key | Action |
| :--- | :--- | :--- |
| **Dates** | `j` / `↓` | Move selection down |
| **Dates** | `k` / `↑` | Move selection up |
| **Dates** | `Enter` / `l` / `→` | Switch focus to Tasks column |
| **Dates** | `D` / `Shift+d` | Clear all completed tasks for selected date |
| **Tasks** | `h` / `←` | Switch focus back to Dates column |
| **Tasks** | `j` / `↓` | Move selection down |
| **Tasks** | `k` / `↑` | Move selection up |
| **Tasks** | `J` / `Shift+j` | Move selected task down one position (manual sort only) |
| **Tasks** | `K` / `Shift+k` | Move selected task up one position (manual sort only) |
| **Tasks** | `Space` / `Enter` | Toggle task completion |
| **Tasks** | `e` | Edit selected task text |
| **Tasks** | `p` | Cycle task priority |
| **Tasks** | `n` | Edit task note |
| **Tasks** | `t` | Reschedule task to another date |
| **Tasks** | `m` | Move selected task to Today (resets to unfinished) |
| **Tasks** | `d` / `Delete` | Delete selected task |
| **Tasks** | `D` / `Shift+d` | Clear all completed tasks for selected date |

### Multi-Line Note Editor (`n`)
| Key | Action |
| :--- | :--- |
| `Enter` | Save note (cleared if empty) |
| `Alt+Enter` / `Ctrl+j` | Insert newline |
| `Esc` | Cancel editing without saving changes |
| `←` / `→` / `↑` / `↓` | Navigate cursor across lines and columns |
| `Home` / `End` | Jump to start / end of current line |
| `Backspace` / `Del` | Delete characters (max 2000 characters) |

### Inline Input Prompt (Add / Edit / Filter / Reschedule / Recurrence)
| Key | Action |
| :--- | :--- |
| `Enter` | Apply input and return to Normal mode |
| `Esc` | Cancel prompt and return to Normal mode |
| `Backspace` / `Del` | Delete characters |
| `←` / `→` | Move text cursor |
| `Home` / `End` | Jump to start / end of input |

---

## Quick-Add Syntax

Slate features a shared, pure quick-add parser used identically in the TUI inline prompt and the `slate add` CLI command. Tokens are whitespace-delimited words that can appear anywhere in the input:

| Token Type | Syntax | Examples | Notes |
| :--- | :--- | :--- | :--- |
| **Dates (`@`)** | `@today`, `@tomorrow`, `@mon`..`@sun`, `@monday`..`@sunday`, `@YYYY-MM-DD`, `@MM-DD`, `@+Nd`, `@+Nw` | `@tomorrow`, `@fri`, `@2026-11-15`, `@11-15`, `@+3d`, `@+2w` | Weekdays resolve to the next occurrence strictly after today. `N` is clamped to `1..365`. |
| **Recurrence (`@`)** | `@daily`, `@weekdays`, `@weekly` | `@daily`, `@weekdays`, `@weekly` | Creates a recurring template. `@weekly` repeats on the weekday of the task's date. |
| **Priority** | `!`, `!!` | `!`, `!!` | Must be exactly `!` (priority 1) or `!!` (priority 2). Trailing exclamation points in words like `Great!` are regular text. |
| **Tags** | `#<name>` | `#work`, `#home-repair`, `#tax_2026` | First character must be a letter, followed by letters, digits, `_`, or `-`. Lowercased, max 32 chars, max 5 tags per task. `#12` is treated as plain text. |
| **Escapes** | `\` prefix | `\@fri`, `\#urgent`, `\!` | Removes the backslash and preserves the word verbatim as plain task text. Unknown `@words` (like `hello@example.com`) are also preserved as text. |

### Validation & Errors
- **Past dates**: Dates strictly before today are rejected (`date cannot be in the past`).
- **Conflicting dates**: Specifying two distinct date tokens is rejected.
- **Date + Recurrence conflicts**: A date token combined with recurrence is allowed only if the date matches the recurrence rule (e.g. `@fri @weekly` is valid; `@daily @fri` is rejected with a clear explanation).
- **Subtask constraints**: Subtasks inherit their parent task's scheduled date and reject date and recurrence tokens.
- **Edit Prompt Round-Trip**: Pressing `e` on an existing task re-serializes its metadata into quick-add tokens (e.g. `Buy groceries @2026-10-30 ! #food`) allowing in-place editing.

---

## Command Line

Slate features a first-class scriptable command-line interface. You can add tasks, mark them complete, and inspect today's progress without touching the terminal state.

### Running with No Arguments
- **Interactive Terminal (TTY)**: Running `slate` with no arguments launches the full TUI.
- **Non-Interactive (Piped / Scripted)**: If `stdout` is not a TTY, Slate refuses to start the TUI, prints `slate: not a terminal (try "slate list")` to `stderr`, and exits with code `1`.

### Core CLI Design Principles
- **Clean Standard Streams**: `stdout` carries data only (ready for UNIX piping). Warnings, errors, and hints go exclusively to `stderr`.
- **Zero Terminal State Alteration**: Subcommands never enter raw mode, never switch to the alternate screen, and never enable mouse capture.
- **Broken-Pipe Safe**: Commands like `slate list | head -n 1` exit cleanly without panic or broken pipe errors (`EPIPE`).
- **Data Safety**: Every mutating command executes load -> modify -> atomic save (via temporary file and atomic rename) under an advisory sidecar file lock (`todos.json.lock`).
- **TTY & Color Detection**: Respects `NO_COLOR` and ASCII symbols settings. Piped output is always plain text without ANSI escape sequences.

---

### Global Options

These options apply to both the TUI and all CLI subcommands. Note that the task data file is always located at the OS default location (see [Data Storage Locations](#data-storage-locations)):

| Option | Environment Variable | Description |
| :--- | :--- | :--- |
| `--config <PATH>` | `SLATE_CONFIG` | Path to TOML configuration file. Overrides `SLATE_CONFIG` and the OS default. |
| `-V`, `--version` | | Print version. |
| `-h`, `--help` | | Print help and subcommand usage examples. |

---

### Subcommands Summary

| Command | Arguments & Options | Description |
| :--- | :--- | :--- |
| `slate add <TEXT>...` | `[--date <DATE>]` `[--parent <N>]` `[-q \| --quiet]` | Adds a new task using the shared quick-add parser (`@date`, `!`/`!!`, `#tag`, `@recurrence`). Use `--parent <N>` to add as a subtask under task N on that date. Pass `-` to read from stdin. Conflicting `--date` and `@date` tokens exit 2. |
| `slate done <N>...` | `[--date <DATE>]` `[--match <TEXT>]` | Marks tasks done. Supports top-level index `N` or subtask index `N.M`. Completing a parent marks all its subtasks done. |
| `slate undone <N>...` | `[--date <DATE>]` `[--match <TEXT>]` | Marks tasks unfinished. Un-doing a subtask reopens a done parent. |
| `slate rm <N>...` | `[--date <DATE>]` `[--match <TEXT>]` | Deletes tasks. Deleting a parent cascades to all its subtasks. Supports `N` and `N.M`. |
| `slate list` | `[--date <DATE>]` `[--all]` `[--pending]` `[--limit <N>]` `[--tag <TAG>]` `[--min-priority <1\|2>]` `[--upcoming]` | Lists tasks for a day, across all days (`--all`), or on future days (`--upcoming`, ascending). Filters by `--tag` and `--min-priority`. Read-only on disk (materializes recurrence in memory without modifying the file). |
| `slate today` | | Summary for today: weekday header, progress bar (counting top-level tasks only), and tasks. Read-only on disk. |

---

### Addressing Tasks & Dates

#### Addressing by Index or Substring
Tasks in mutating commands (`done`, `undone`, `rm`) are addressed by their **1-based index** in that day's list, or by `--match <TEXT>`:
- **Top-level Tasks (`N`)**: Addressed by 1-based sequential integers (e.g. `slate done 1 3`).
- **Subtasks (`N.M`)**: Addressed by hierarchical dot notation, where `N` is the parent's index and `M` is the subtask's 1-based index (e.g. `slate done 1.1`, `slate rm 2.3`).
- **Substring Match (`--match <TEXT>`)**: Case-insensitive substring matching against task text. Matches top-level tasks and subtasks. If multiple tasks match, lists candidates with their `N` and `N.M` indices.
- **Atomic Validation**: Multi-index commands validate all indices before applying mutations. If any index is invalid, no tasks are modified.
- **Stable Multi-Index Removal**: In `slate rm 1 2`, indices are resolved against the list as it existed *before* any deletions occur, correctly removing the original first and second tasks.

#### Date Syntax (`--date`)
The `--date` option accepts:
- `today` *(default)*
- `yesterday`
- `tomorrow`
- `YYYY-MM-DD` (e.g. `2026-10-27`)

---

### Output Format

`slate list` and `slate today` output a clean, human-readable format. The output shows priority (`!`, `!!`), repeat markers (`↻`, ASCII `~`), subtask progress (`0/2`), tags (`#tag`), and note markers (`≡`, ASCII `=`). When redirected or piped, output uses the same layout without ANSI escape sequences.

#### `slate list`
```text
  1  ○  ! Prepare presentation slides 0/2 #work
    1.1  ` [ ]  Outline agenda
    1.2  ` [ ]  Design architecture diagram
  2  ✓  ~ !! Morning team sync #standup
  3  ○  Order office supplies #admin =
  4  ✓  Review pull request           done 11:53
```
*(Uses active theme symbols and time format. When `--all` or `--upcoming` is used, a date header precedes each day's group).*

#### `slate today`
Prints the formatted weekday header, done/total progress indicator (counting top-level tasks only), and today's task list:
```text
Friday, 23 October
2 of 3 done  ███████░░░

  1  ○  ! Prepare presentation slides 0/2 #work
    1.1  ` [ ]  Outline agenda
    1.2  ` [ ]  Design architecture diagram
  2  ✓  ~ !! Morning team sync #standup
  3  ✓  Review pull request           done 11:53
```

---

### Exit Codes

Slate uses standard, predictable exit codes for scripting:

| Code | Meaning | Description |
| :---: | :--- | :--- |
| `0` | **Success** | Command completed successfully. |
| `1` | **Runtime / I/O Error** | Unreadable storage file, corrupt JSON database, lock acquisition timeout, or non-terminal environment without a subcommand. |
| `2` | **Usage Error** | Invalid date format, empty task text, or missing arguments. |
| `3` | **No Such Task** | Specified task index out of range, or `--match` found 0 matches or multiple ambiguous candidates. |

---

### Concurrency & Advisory Locking

- **Sidecar Advisory Lock**: Every mutating CLI command wraps its load-modify-save sequence in an exclusive lock on `<datafile>.lock` using `fs2`. If another process holds the lock, Slate retries for up to 3 seconds before failing with exit code `1`. The lock is released on all exit paths, including errors.
- **Important Limitation (TUI Memory Model)**: The interactive TUI holds its task dataset in memory and writes back to disk on save. If tasks are added or modified via the CLI while the interactive TUI is actively running, the next write from the TUI will overwrite the CLI's changes (live reloading is planned for a future phase). **Avoid running mutating CLI commands while the interactive TUI is actively open.**

---

## Codebase Architecture

Slate is organized into small, focused modules under `src/`:

```text
src/
├── main.rs         # Entrypoint, terminal guard, TUI event loop, and CLI dispatcher
├── lib.rs          # Library root declaring public modules and path resolution helpers
├── cli.rs          # Clap derive definitions and command-line arguments
├── commands.rs     # CLI command handlers (add, done, undone, rm, list, today)
├── output.rs       # Output formatters and broken-pipe-safe writer
├── parser.rs       # Shared quick-add parser (dates, recurrence, priority, tags, escapes)
├── store.rs        # Shared domain operations on task collections, row projection, recurrence
├── app.rs          # App state, task mutations, undo/redo stack, action dispatch, selection by id
├── config.rs       # Runtime TOML configuration, strftime/color validation, and OS path resolution
├── keymap.rs       # Action enum, key chord parser, conflict detection, arrows mode, dynamic hints
├── model.rs        # Data models (Todo schema v2, Recurring, Filter, Row, SortMode, View)
├── storage/        # Storage subsystem:
│   ├── mod.rs      # Atomic file loading and saving, external change detection, try_load_modify_save
│   ├── migrate.rs  # Stepwise schema migrations (v0 -> v1 -> v2) and version detection
│   ├── backup.rs   # Rolling backups engine, snapshot rotation, and pruning
│   └── lock.rs     # Sidecar advisory file locking (3-second timeout retry loop)
├── theme.rs        # Runtime Theme struct, presets (teal, orange, lavender, mono), and 256-color fallback
└── ui.rs           # Ratatui rendering, Symbols (Unicode/ASCII), HitArea tracking, detail pane
```

### Module Responsibilities

1. **[`src/cli.rs`](src/cli.rs)**:
   Defines the command-line interface using `clap` derive, including global options (`--config`), subcommands, arguments, and after-help example blocks.
2. **[`src/commands.rs`](src/commands.rs)**:
   Implements command execution for `add`, `done`, `undone`, `rm`, `list`, and `today`, including 1-based index validation, hierarchical `N.M` subtask addressing, case-insensitive `--match` query resolution, and structured exit codes. Read commands are strictly non-mutating on disk.
3. **[`src/parser.rs`](src/parser.rs)**:
   Pure tokenizer and parser for quick-add syntax, supporting dates (`@tomorrow`, `@fri`, `@+3d`), recurrence (`@daily`, `@weekdays`, `@weekly`), priority (`!`, `!!`), tags (`#tag`), and literal escapes (`\`). Shared between CLI and TUI.
4. **[`src/output.rs`](src/output.rs)**:
   Generates human-readable list and today formatted output including priority, repeat markers, subtask progress, tags, and note markers. Implements `SafeWriter` to handle closed pipes (`EPIPE`) gracefully without panics.
5. **[`src/store.rs`](src/store.rs)**:
   Pure domain logic operating on `Vec<Todo>` and `Vec<Recurring>` (`visible_rows` projection, `materialize_recurring`, `set_task_done` completion cascades, `delete_task`, `edit_task`). Shared between interactive TUI and scriptable CLI.
6. **[`src/config.rs`](src/config.rs)**:
   Loads configuration from `config.toml` (or `SLATE_CONFIG` / `--config`), validates settings, and provides per-field fallback with non-blocking transient warnings on bad input.
7. **[`src/keymap.rs`](src/keymap.rs)**:
   Manages keymappings through the `Action` enum, parses key chord strings, enforces unbinding protection on `Ctrl+c`, detects conflicts, and generates dynamic hints.
8. **[`src/theme.rs`](src/theme.rs)**:
   Runtime `Theme` supporting presets (`teal`, `orange`, `lavender`, `mono`), custom accent color overrides, automatic ANSI-256 fallback, and `NO_COLOR` compliance.
9. **[`src/ui.rs`](src/ui.rs)**:
   Layout rendering, `Symbols` table (Unicode vs ASCII fallback), detail pane, small terminal notice guard, and mouse hit-testing areas.
10. **[`src/storage/`](src/storage/)**:
   Provides crash-resilient data persistence:
   - **`storage/mod.rs`**: Resolves OS paths, atomic save engine (tempfiles, fsync, rename with retries), and locked load-modify-save helper.
   - **`storage/migrate.rs`**: Stepwise schema migrations (v0 -> v1 -> v2) creating `.v0.bak` and `.v1.bak` backups.
   - **`storage/backup.rs`**: Rolling backup system creating timestamped snapshots in `backups/`.
   - **`storage/lock.rs`**: Advisory file locking via `todos.json.lock` with a 3-second timeout loop.
11. **[`src/app.rs`](src/app.rs)**:
    Maintains interactive TUI memory, selection by task ID, undo/redo stacks, note editing buffer, filter state, and routes input events.
12. **[`src/main.rs`](src/main.rs)**:
    Thin dispatcher: validates TTY state, initializes terminal raw mode when running interactively, or routes directly to CLI subcommands without touching terminal state.

---

## Configuration

Slate is completely zero-config out of the box. Without a configuration file, Slate runs with default styling and vim-style keybindings. Slate never creates or writes a configuration file automatically.

To customize Slate, create a `config.toml` file in your operating system's standard configuration directory:

- **Linux**: `~/.config/slate/config.toml`
- **macOS**: `~/Library/Application Support/slate/config.toml`
- **Windows**: `%APPDATA%\slate\config.toml` *(e.g. `C:\Users\<Username>\AppData\Roaming\slate\config.toml`)*

You can override the config file location using the `SLATE_CONFIG` environment variable:
```bash
export SLATE_CONFIG="$HOME/dotfiles/slate.toml"
```

A fully commented example configuration file is included in the repository root: [`config.example.toml`](config.example.toml).

### Precedence (Lowest to Highest)

1. **Built-in Defaults**: Hardcoded safe defaults.
2. **Theme Preset**: Selected preset colors (`teal`, `orange`, `lavender`, `mono`).
3. **`config.toml` Values**: User options specified in the config file.
4. **Environment Variables**: `NO_COLOR`, `SLATE_CONFIG`, and `SLATE_ASCII`.

### Fault-Tolerant Fallbacks

Slate is crash-proof: a missing file, invalid TOML syntax, unknown key, invalid color format, or invalid strftime string will **never crash Slate**. Instead, Slate falls back to the safe default for that specific field and displays a dim, non-intrusive one-line warning in the footer status area.

### Configuration Schema

| Setting | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `theme` | string | `"teal"` | Theme preset: `"teal"`, `"orange"`, `"lavender"`, or `"mono"`. |
| `accent` | string | *(preset accent)* | Optional accent override: hex (`"#4ebaaf"`) or ANSI name (`"cyan"`, `"lightmagenta"`). |
| `date_format` | string | `"%A, %d %B"` | strftime format for Today and selected date headers (e.g. `"Friday, 23 October"`). |
| `short_date_format` | string | `"%a %d %b"` | strftime format for History dates column (e.g. `"Mon 19 Oct"`). |
| `time_format` | string | `"24h"` | Completion timestamp format in History: `"24h"` (`"done 14:32"`) or `"12h"` (`"done 2:32 pm"`). |
| `week_start` | string | `"monday"` | Week start day: `"monday"`, `"sunday"`, or `"saturday"`. *(Validated and stored; the week summary view is planned for a later phase so no current view consumes this setting yet).* |
| `[ui]` | table | | UI display settings. |
| `ui.symbols` | string | `"unicode"` | Symbol mode: `"unicode"` (`○`, `✓`, `█`) or `"ascii"` (`[ ]`, `[x]`, `#`). |
| `ui.show_history` | boolean | `true` | When `false`, hides the History tab and view entirely. `Tab`/`Shift+Tab` do nothing, and History shortcuts are hidden. Historical data remains safely preserved on disk. |
| `ui.show_upcoming` | boolean | `true` | When `false`, hides the Upcoming tab and view entirely. Future scheduled tasks remain safely preserved on disk. |
| `ui.detail_pane` | boolean | `false` | When `true`, shows task details (full wrapped text, note, priority, tags, recurrence) beneath the list. Auto-hides when terminal height is below 20 rows. |
| `ui.mouse` | boolean | `false` | When `true`, enables mouse interaction. When `false`, mouse capture is not enabled so terminal text selection remains uninterrupted. |
| `[tasks]` | table | | Task display and sorting settings. |
| `tasks.sort` | string | `"manual"` | Task sort mode: `"manual"` (user-defined order) or `"priority"` (high priority first). Sorting is a view projection and never alters stored order. |
| `tasks.show_tags` | boolean | `true` | Show or hide dim `#tag` labels inline in task rows (automatically hidden on terminals narrower than 60 columns). |
| `[progress]` | table | | Progress bar styling. |
| `progress.style` | string | `"block"` | Bar style: `"block"` (`█░`), `"line"` (`━─`), `"dots"` (`●○`), `"ascii"` (`#-`), or `"none"` (hide bar). |
| `progress.width` | integer | `10` | Progress bar column width (clamped to range `4..=40`). Safe against zero tasks. |
| `progress.show_count` | boolean | `true` | Show or hide completion counter text (`"2 of 3 done"`). |
| `[keys]` | table | | Keybinding settings. |
| `keys.mode` | string | `"default"` | Navigation mode: `"default"` (vim-style `j`/`k`/`h`/`l` + arrows) or `"arrows"` (arrows, PageUp/PageDown, Home/End only). |
| `keys.<action>` | array | *(varies)* | Per-action key overrides. Replaces the default keys for that action. |

### Keybinding Customization

To customize keybindings, define action overrides under `[keys]`:

```toml
[keys]
mode = "default" # "default" | "arrows"

# Overrides replace the defaults for that action
quit = ["q", "ctrl+c"]
add = ["a", "ctrl+n"]
toggle = ["space", "enter"]
delete = ["d", "delete"]
reorder_up = ["alt+up", "K"]
reorder_down = ["alt+down", "J"]
```

#### Key String Syntax
- **Single characters**: `"a"`, `"e"`, `"j"`, `"k"`, `"J"`, `"?"`
- **Named keys**: `"enter"`, `"esc"`, `"space"`, `"tab"`, `"backtab"`, `"up"`, `"down"`, `"left"`, `"right"`, `"home"`, `"end"`, `"pageup"`, `"pagedown"`, `"delete"`, `"backspace"`
- **Modifiers**: `"ctrl+"`, `"alt+"`, `"shift+"` (case-insensitive, e.g. `"ctrl+d"`, `"Shift+Up"`)

#### Safety & Conflict Rules
- `Ctrl+c` always quits and cannot be unbound.
- Overriding an action **replaces** that action's default bindings.
- Conflicting bindings (assigning the same key to two different actions) generate a transient footer warning, keep the first assigned action, and ignore the second.
- Text input prompt keys (`Enter`, `Esc`, arrows, `Backspace`, `Home`/`End`) and the help overlay close keys remain fixed for typing.

### Opt-In Mouse Support

When `[ui] mouse = true` is enabled in `config.toml`:
- **Left click** on a task row selects it.
- **Left click** directly on a task's checkbox (`○` / `✓` / `[ ]` / `[x]`) toggles completion.
- **Left click** on the `Today` or `History` tab switches the active view.
- In **History view**, left clicking a date selects it; left clicking a task in the right pane focuses the Tasks column and selects it.
- **Mouse scroll wheel** moves selection up or down by one item.
- Clicks outside interactive elements or on empty lists are safe no-ops.
- Mouse events are automatically ignored while typing in the input prompt or viewing the help overlay.
- Mouse capture is automatically released on exit and in the panic hook, ensuring your terminal is never left in an unselectable state.

### Environment Variables

| Variable | Description |
| :--- | :--- |
| `NO_COLOR` | When set to any non-empty value (per [no-color.org](https://no-color.org)), disables all color output. Slate switches to monochrome mode using text modifiers (bold, dim, underline) only. |
| `SLATE_ASCII` | When set (`SLATE_ASCII=1`), forces ASCII symbol mode (`[ui] symbols = "ascii"` and `progress.style = "ascii"`). |
| `SLATE_CONFIG` | Overrides the default config file path (useful for testing or dotfiles management). |

---

## Theming

Slate includes four built-in theme presets and supports custom accent overrides in `config.toml`:

```toml
theme = "teal"       # teal | orange | lavender | mono
accent = "#4ebaaf"   # optional override (hex or ANSI name)
```

- **`teal`** *(default)*: Calm, restrained soft teal (`Rgb(78, 186, 175)`) with warm orange overdue warnings.
- **`orange`**: Warm, energetic amber accent (`Rgb(228, 140, 75)`).
- **`lavender`**: Soft purple pastel accent (`Rgb(180, 140, 220)`).
- **`mono`**: Pure monochrome without any hue. Uses your terminal's natural foreground color with bold for selection, dim for secondary text, and underline/reverse for emphasis. Overdue and error states remain distinguishable through styling and symbols, not color alone.

### Terminal Compatibility & 256-Color Fallback

If your terminal does not advertise 24-bit truecolor (i.e. `COLORTERM` is not `truecolor` or `24bit`), Slate automatically converts 24-bit RGB colors to the nearest matching ANSI 256-color palette index.

---

## Data Storage Locations

Slate stores your tasks as human-readable JSON using your operating system's standard application data directory:

- **Linux**:
  ```text
  ~/.local/share/slate/todos.json
  ```
- **macOS**:
  ```text
  ~/Library/Application Support/slate/todos.json
  ```
- **Windows**:
  ```text
  %APPDATA%\slate\todos.json
  (e.g., C:\Users\<Username>\AppData\Roaming\slate\todos.json)
  ```

### Storage Directory Layout

Inside your Slate data directory, you will find:

```text
slate/
├── todos.json                       # Active task database (Schema version 2)
├── todos.json.lock                  # Advisory sidecar lock file (held only during saves)
├── todos.v0.bak                     # One-time legacy backup (created when migrating v0 -> v1)
├── todos.v1.bak                     # One-time legacy backup (created when migrating v1 -> v2)
└── backups/                         # Rolling snapshots directory (up to 10 newest kept)
    ├── todos-20261027-143000.json
    └── todos-20261027-144500.json
```

### On-Disk Schema & Compatibility (Schema Version 2)

Slate uses an explicit versioned JSON schema on disk:
```json
{
  "version": 2,
  "next_id": 5,
  "todos": [
    {
      "id": 1,
      "text": "Prepare presentation slides",
      "created_date": "2026-10-27",
      "done": false,
      "done_at": null,
      "priority": 1,
      "tags": ["work"],
      "note": "Include benchmark charts",
      "parent": null,
      "recurring_id": null
    },
    {
      "id": 2,
      "text": "Outline agenda",
      "created_date": "2026-10-27",
      "done": false,
      "done_at": null,
      "priority": 0,
      "tags": [],
      "note": null,
      "parent": 1,
      "recurring_id": null
    }
  ],
  "recurring": [
    {
      "id": 3,
      "text": "Morning team sync",
      "tags": ["standup"],
      "priority": 2,
      "rule": "daily",
      "anchor": "2026-10-26",
      "last_materialized": "2026-10-27"
    }
  ],
  "focus": 1
}
```

- **Stepwise Migration (v1 -> v2)**: Older v1 files are automatically migrated by assigning sequential IDs in stable file order, allocating `next_id`, and setting defaults for new fields. Before the first write in v2 format, a byte-for-byte copy is preserved as `todos.v1.bak` (which is never overwritten).
- **In-Memory Repair on Load**:
  - A subtask whose parent is missing or located on a different date is promoted to a top-level task.
  - Subtask depth beyond one level is flattened to depth 1.
  - If a done parent has an undone child, the parent is reopened.
  - If `focus` references a task ID that does not exist, it is safely cleared to `null`.
  - Duplicate task IDs are rejected as a hard load error with the file left untouched.
- **Future Versions**: If `todos.json` was created by a newer version of Slate with a higher schema version, Slate refuses to overwrite it and cleanly exits with an upgrade notification.

### Documented Limitations

- **Recurring Tasks in Upcoming**: Recurring tasks are template rules rather than infinite pre-expanded items; they are not previewed in the Upcoming view until materialized on or after their scheduled day.
- **Single-Level Subtasks**: Subtasks are strictly one level deep. Subtasks cannot have their own child subtasks.
- **Top-Level Counting**: The progress bar, completion counter (`2 of 4 done`), and overdue indicators count top-level tasks only. Completing all subtasks does not auto-complete the parent.
- **TUI Memory Model**: The interactive TUI holds its task dataset in memory. Avoid running mutating CLI commands while the interactive TUI is actively open.

### Concurrency & External Changes

- **Advisory Sidecar Locking**: Slate locks `todos.json.lock` only for the duration of a write operation, preventing simultaneous processes from corrupting data. Unique task IDs are allocated under this lock.
- **External Changes**: Before saving, Slate checks the file's fingerprint (mtime + size). If the file changed on disk, Slate automatically creates a safety snapshot in `backups/` before writing and displays a transient footer notice: `Data changed on disk · previous version backed up`.

