# Slate

A minimal terminal todo app for everyday task tracking.

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

```text
  Slate    Today   Upcoming   History   Summary
  ────────────────────────────────────────────────────────────────────────

  › Tomorrow           0/2│  Saturday, 24 October   •   0 of 2 done
    Mon 26 Oct         0/1│
                          │  › ○ Plan sprint backlog #work
                          │    ○ !! Doctor appointment

  enter/l tasks  ·  tab history  ·  ? help  ·  q quit
```

## Highlights

- Quiet, unboxed terminal interface built with Ratatui using subtle accents and thin rules.
- Plain JSON file storage using schema version 2 with atomic temp-file saves.
- Automated rolling timestamped backups keeping the 10 newest snapshots in a `backups/` directory.
- Advisory sidecar locking (`todos.json.lock`) to prevent concurrent process collisions.
- Live file reload that detects external changes and refreshes without interrupting open input prompts.
- Keyboard-first workflow supporting remappable keys, vim-style navigation, and an arrow-only mode.
- Non-interactive CLI for scripting and shell operations (`add`, `list`, `done`, `undone`, `rm`, `today`).
- Dedicated views for today's agenda, upcoming scheduled dates, past history, and summary heatmaps.

## Installation

### Prerequisites

- A [Rust toolchain](https://rustup.rs) (version 1.85 or newer).
- A terminal with UTF-8 support.

### Build and Install

Clone the repository and install the binary with Cargo:

```bash
git clone https://github.com/LakshThakre/Slate.git
cd Slate
cargo install --path .
```

Ensure Cargo's binary directory is on your system `PATH`:

- **Linux and macOS**: Add `export PATH="$HOME/.cargo/bin:$PATH"` to your `~/.bashrc` or `~/.zshrc`.
- **Windows**: Add `%USERPROFILE%\.cargo\bin` to your User Environment variables, or restart your terminal.

Run Slate:

```bash
slate
```

## Quick Start

### Core Keys

| Key | Action |
| :--- | :--- |
| `a` | Add a new task (opens inline prompt) |
| `Space` / `Enter` | Toggle task completion |
| `Tab` / `Shift+Tab` | Cycle through Today, Upcoming, History, and Summary |
| `e` | Edit selected task text |
| `d` / `Delete` | Delete selected task |
| `u` / `Ctrl+r` | Undo / redo last action |
| `?` | Toggle help overlay |
| `q` | Quit Slate |

### Command Line

You can inspect and modify tasks directly from your shell without opening the TUI:

```console
$ slate add "Buy milk"
Added: Buy milk

$ slate list
   1  ○  Buy milk

$ slate done 1

$ slate today
Saturday, 31 October
1 of 1 done  ██████████

   1  ✓  Buy milk           done 21:05
```

## Keyboard Shortcuts

The following default shortcuts are active in normal view mode. All keys can be rebound in `config.toml`. See [docs/FEATURES.md](docs/FEATURES.md#full-keyboard-reference) for the complete reference.

| Key | Context | Action |
| :--- | :--- | :--- |
| `j` / `k` or `Down` / `Up` | All views | Move cursor down / up |
| `A` | Today / Upcoming | Add subtask under selected task |
| `p` | Today / Upcoming | Cycle priority (none, `!`, `!!`) |
| `t` | Today / Upcoming | Reschedule task to another date |
| `r` | Today / Upcoming | Set recurrence rule (`daily`, `weekdays`, `weekly`, `off`) |
| `P` | Today / Upcoming | Toggle focus task pin |
| `n` | Today / Upcoming | Open multi-line note editor |
| `i` | Today / Upcoming | Toggle task detail pane |
| `v` / `V` | Today / Upcoming | Toggle selection mark / mark all |
| `J` / `K` | Today / Upcoming | Reorder task down / up (manual sort only) |
| `c` | Today | Carry over unfinished tasks from previous active day |
| `D` | Today / History | Clear all completed tasks for selected date |
| `/` | Global | Search tasks across all dates |
| `:` | Global | Jump to a specific date expression |
| `s` | Global | Cycle sort order (`manual` / `priority`) |
| `f` | Global | Filter by tag or priority |
| `w` | Summary | Toggle summary range between Week and Month |
| `[` / `]` | Summary | Navigate to previous / next period |

## Configuration

Configuration is optional. Slate looks for a TOML configuration file at the following path:

- **Linux**: `~/.config/slate/config.toml`
- **macOS**: `~/Library/Application Support/slate/config.toml`
- **Windows**: `%APPDATA%\slate\config.toml`

You can also point the `SLATE_CONFIG` environment variable to any file path.

### Minimal Example

```toml
theme = "teal"
date_format = "%A, %d %B"
time_format = "24h"
week_start = "monday"

[ui]
symbols = "unicode"
detail_pane = false
mouse = false

[tasks]
sort = "manual"
```

For all configuration keys, theme options, and keybinding syntax, see [config.example.toml](config.example.toml) and [docs/FEATURES.md](docs/FEATURES.md#customization).

## Your Data

Slate stores tasks in a human-readable JSON file:

- **Linux**: `~/.local/share/slate/todos.json`
- **macOS**: `~/Library/Application Support/slate/todos.json`
- **Windows**: `%APPDATA%\slate\todos.json`

Safety guarantees:
- **Atomic saves**: Slate writes updates to a temporary file (`.todos.json.tmp.<pid>.<nanos>`), flushes it to disk, and renames it over the existing file.
- **Locking**: An advisory lock file (`todos.json.lock`) is acquired before writing, preventing data races between CLI commands and running TUI instances.
- **Automatic backups**: Timestamped snapshots (`todos-YYYYMMDD-HHMMSS.json`) are stored in `backups/` upon session start, every 10 minutes during active use, and when external changes are detected. Slate retains the 10 newest backups.
- **Data integrity**: Slate never overwrites an unreadable or corrupt file. If the file cannot be parsed or was written by a newer schema version, Slate preserves the current in-memory view and reports the issue.

## Documentation

- [Feature and Architecture Guide](docs/FEATURES.md): Comprehensive reference covering all task features, quick-add tokens, CLI subcommands, internal architecture, and design decisions.
- [Configuration Example](config.example.toml): Documented example covering every setting, default value, and override option.

## Project Status

Slate is a personal open-source project written in Rust using Ratatui and crossterm. The minimum supported Rust version (MSRV) is 1.85, and the test suite passes 243 automated tests across unit, terminal rendering, and CLI integration suites.
