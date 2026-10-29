# Slate

[![CI](https://github.com/LakshThakre/Slate/actions/workflows/ci.yml/badge.svg)](https://github.com/LakshThakre/Slate/actions/workflows/ci.yml)

> A minimal, calm, and distraction-free terminal todo app built in Rust.

Slate is designed in the spirit of modern, quiet developer tools like Claude Code and Codex CLI. It discards heavy boxed borders in favor of generous whitespace, subtle padding, thin divider rules, and a restrained color palette that respects your terminal's natural theme.

---

## Features

- **Quiet, Minimal Interface**: Clean layout without heavy box borders, using subtle accents and breathable spacing.
- **Cross-Platform**: Works smoothly across Linux, macOS, and Windows.
- **Runtime Theming & Presets**: Choose between `teal` (default), `orange`, `lavender`, or pure `mono` presets, or specify any custom hex (`#4ebaaf`) or ANSI accent color. Automatic 256-color fallback on terminals without 24-bit truecolor.
- **`NO_COLOR` & ASCII Compatibility**: Honors `NO_COLOR` per no-color.org (pure monochrome with text modifiers) and supports pure ASCII symbols fallback (`SLATE_ASCII=1` or `[ui] symbols = "ascii"`).
- **Customizable Progress Bar**: Tailor progress bar style (`block`, `line`, `dots`, `ascii`, or `none`), width (clamped to `4..=40`), and done counters.
- **Remappable Keybindings & Arrows Mode**: Customize any keybinding via `config.toml` or switch navigation to arrow keys with `keys.mode = "arrows"`. Footer hints and help overlay update automatically.
- **Opt-In Mouse Support**: Optional mouse capture for clicking task rows, toggling checkboxes, switching tabs, and mouse-wheel scrolling (`[ui] mouse = true`).
- **Today & History Views**:
  - **Today View**: See today's tasks with a friendly weekday header and a slim block progress bar (`████░░░░░░`). Shows subtle overdue and carry-over indicators.
  - **History View**: Browse past days by date (`Today`, `Yesterday`, `Mon 19 Oct`) with task counts, completion timestamps (`done 14:32`), and overdue indicators. Can be hidden entirely with `ui.show_history = false`.
- **Task Text Editing (`e`)**: Edit tasks in Today and History views with an inline prompt pre-filled with existing text.
- **Full Undo / Redo (`u`, `Ctrl+r`)**: In-memory snapshot stack (up to 50 operations) supporting all mutations (add, edit, toggle, delete, reorder, clear completed, carry over, move) with transient status feedback.
- **Carry Over Unfinished Tasks (`c`)**: Subtle hint line on startup or rollover showing unfinished tasks from previous days with one-key carry-over into Today.
- **Task Reordering (`J` / `K`)**: Move tasks up or down within the same day with `Shift+j` / `Shift+k`.
- **Clear Completed (`D` / `Shift+d`)**: Clear all completed tasks for today or the selected historical date in a single action.
- **Jump Navigation (`g`, `G`, `Ctrl+d`, `Ctrl+u`)**: Quick jumps to the top, bottom, or half-page scroll in any focused list.
- **Help Overlay (`?`)**: Minimal, non-intrusive modal overlay displaying all grouped keybindings.
- **Overdue Indicators**: Past days with unfinished tasks are highlighted with warm warning markers in History and a badge in the Today header.
- **Move Tasks Between Days (`m`)**: Move historical tasks directly into Today with completion reset to unfinished.
- **Prompt-Style Input**: Tap `a` to open a modern inline CLI prompt right at the bottom of the screen (`› What needs doing today?`).
- **Crash-Proof Hardened Storage**: Saves automatically after every change using unique PID-tagged tempfiles, fsync, and atomic rename with retry logic. Exclusive advisory locking (`todos.json.lock`) prevents write collisions.
- **Automated Rolling Backups**: Automatically takes timestamped snapshots in `backups/` before modifying data (first save of session, 10-minute intervals, or external modifications), retaining the 10 newest backups.
- **Schema Versioning & Stepwise Migrations**: On-disk versioning (`{"version": 1, "todos": [...]}`) with backward-compatible migrations from legacy v0 formats, creating a one-time safety backup (`todos.v0.bak`).
- **Terminal Resize & Small Screen Guard**: Dynamically adapts to terminal dimensions, clamping selections on resize. Renders a quiet centered notice (`Terminal too small · need 40×8, have W×H`) when below 40×8 while freezing mutations and preserving half-typed input until restored.
- **Midnight Daily Rollover**: Automatically advances `today` at local midnight with zero idle CPU spin. Tasks from yesterday slide gracefully into History while keeping historical date selections pinned.
- **Lightweight & Fast**: Sub-millisecond startup time, negligible memory footprint, and low CPU usage. Zero config required to run out of the box.

---

## Visual Preview

### Today View
```text
  Slate    Today   History
  ──────────────────────────────────────────────────────────────────────────

  Friday, 23 October    2 of 3 done  ██████░░░░    1 overdue

  › ○ Review project architecture
    ✓ Buy fresh bread
    ✓ Read Rust documentation

  a add  ·  space done  ·  tab history  ·  ? help  ·  q quit
```

### Inline Prompt (while typing `a` or `e`)
```text
  Slate    Today   History
  ──────────────────────────────────────────────────────────────────────────

  Friday, 23 October    2 of 3 done  ██████░░░░

    ○ Review project architecture
    ✓ Buy fresh bread
    ✓ Read Rust documentation

  ──────────────────────────────────────────────────────────────────────────
  › Plan weekend hike                                enter save  ·  esc cancel
```

### History View
```text
  Slate   Today    History
  ──────────────────────────────────────────────────────────────────────────

  › Today           2/3 │  Friday, 23 October   •   2 of 3 done
    Yesterday       1/2 │
    Wed 21 Oct      1/2 │  › ! Review project architecture
                        │    ✓ Buy fresh bread                    done 11:53
                        │    ✓ Read Rust documentation            done 12:10

  h dates  ·  space done  ·  tab today  ·  ? help  ·  q quit
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
| `Tab` / `Shift+Tab` | Toggle between **Today** and **History** views |
| `?` | Toggle help overlay (`Esc`, `?`, or `q` to close) |
| `u` | Undo last action |
| `Ctrl+r` | Redo undone action |
| `g` | Jump to first item in focused list |
| `G` | Jump to last item in focused list |
| `Ctrl+d` | Move half a page down |
| `Ctrl+u` | Move half a page up |
| `q` | Quit Slate (closes help overlay first if open) |

### Today View
| Key | Action |
| :--- | :--- |
| `a` | Add new task (opens inline prompt) |
| `e` | Edit selected task text |
| `Space` / `Enter` | Toggle task completion (`○` ↔ `✓`) |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `J` / `Shift+j` | Move selected task down one position |
| `K` / `Shift+k` | Move selected task up one position |
| `d` / `Delete` | Delete selected task |
| `D` / `Shift+d` | Clear all completed tasks for today |
| `c` | Carry over unfinished tasks from previous day |
| `m` | Move task to Today (notifies "Already in Today") |

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
| **Tasks** | `J` / `Shift+j` | Move selected task down one position |
| **Tasks** | `K` / `Shift+k` | Move selected task up one position |
| **Tasks** | `Space` / `Enter` | Toggle task completion |
| **Tasks** | `e` | Edit selected task text |
| **Tasks** | `m` | Move selected task to Today (resets to unfinished) |
| **Tasks** | `d` / `Delete` | Delete selected task |
| **Tasks** | `D` / `Shift+d` | Clear all completed tasks for selected date |

### Inline Input Prompt (Add / Edit Mode)
| Key | Action |
| :--- | :--- |
| `Enter` | Save task text and return to Normal mode |
| `Esc` | Cancel input and return to Normal mode |
| `Backspace` / `Del` | Delete characters |
| `←` / `→` | Move text cursor |
| `Home` / `End` | Jump to start / end of input |

---

## Command Line

Slate features a first-class scriptable command-line interface. You can add tasks, mark them complete, inspect progress in status bars (tmux, Waybar, Polybar), generate shell completions, and import/export task archives without touching the terminal state.

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

These options apply to both the TUI and all CLI subcommands:

| Option | Environment Variable | Description |
| :--- | :--- | :--- |
| `--data-file <PATH>` | `SLATE_DATA` | Path to storage JSON file. Precedence: flag > `SLATE_DATA` > OS default. Missing parent directories are created on first write; reading a nonexistent file returns no tasks without error. |
| `--config <PATH>` | `SLATE_CONFIG` | Path to TOML configuration file. Overrides `SLATE_CONFIG` and the OS default. |
| `-V`, `--version` | | Print version. |
| `-h`, `--help` | | Print help and subcommand usage examples. |

---

### Subcommands Summary

| Command | Arguments & Options | Description |
| :--- | :--- | :--- |
| `slate add <TEXT>...` | `[--date <DATE>]` `[-q \| --quiet]` | Joins words into one task. Pass `-` to read one task per line from stdin. Trims text and rejects empty tasks (exit code 2). |
| `slate done <N>...` | `[--date <DATE>]` `[--match <TEXT>]` | Marks tasks done and records completion timestamp. Already-done tasks are a no-op. |
| `slate undone <N>...` | `[--date <DATE>]` `[--match <TEXT>]` | Marks tasks unfinished (clears completion timestamp). |
| `slate rm <N>...` | `[--date <DATE>]` `[--match <TEXT>]` | Deletes tasks. Multiple indices are resolved against the original list before any deletion. |
| `slate list` | `[--date <DATE>]` `[--all]` `[--pending]` `[--limit <N>]` `[--plain \| --json]` | Lists tasks for a day or across all days (`--all`, newest first). |
| `slate today` | `[--plain \| --json \| --waybar]` | Summary for today: weekday header, progress bar, and tasks, or custom format. |
| `slate path` | | Prints the resolved data file path. |
| `slate export` | `[--format <FMT>]` `[--from <DATE>]` `[--to <DATE>]` `[--date <DATE>]` `[-o <FILE>]` | Exports tasks to JSON, Markdown, CSV, or plain text (stdout by default). |
| `slate import <FILE\|->` | `[--format <FMT>]` `[--date <DATE>]` `[--replace --yes]` `[--dry-run]` | Imports tasks from JSON, Markdown, CSV, or text (stdin requires `--format`). |
| `slate completions <SHELL>` | `<bash\|zsh\|fish\|powershell>` | Outputs shell completion script to stdout. |

---

### Addressing Tasks & Dates

#### Addressing by Index or Substring
Tasks in mutating commands (`done`, `undone`, `rm`) are addressed by their **1-based index** in that day's list (matching `slate list`), or by `--match <TEXT>`:
- **1-based Index**: Positional numbers are always 1-based indices (e.g. `slate done 1 3`).
- **Substring Match (`--match <TEXT>`)**: Case-insensitive substring matching. Must match exactly one task. If no tasks or multiple tasks match, the command fails with exit code `3` and lists the candidate matches on `stderr`.
- **Atomic Validation**: Multi-index commands validate all indices before applying mutations. If any index is invalid, no tasks are modified.
- **Stable Multi-Index Removal**: In `slate rm 1 2`, indices are resolved against the list as it existed *before* any deletions occur, correctly removing the original first and second tasks.

#### Date Syntax (`--date`)
The `--date` option accepts:
- `today` *(default)*
- `yesterday`
- `tomorrow`
- `YYYY-MM-DD` (e.g. `2026-10-27`)

---

### Output Formats

Every query command (`list`, `today`) supports structured output formats. `--plain`, `--json`, and `--waybar` are mutually exclusive.

#### `slate list`
- **Human (TTY)**:
  ```text
    1  ○  Review project architecture
    2  ✓  Buy fresh bread           done 11:53
  ```
  *(Uses active theme symbols and time format. `--all` prints a dim date header before each day's group).*
- **Plain (`--plain`)**: Tab-separated `<index>\t<done>\t<text>`. Done is represented as `x`, pending as `-`. When `--all` is used, the ISO date is prefixed: `<date>\t<index>\t<done>\t<text>`. Tabs and newlines in task text are sanitized to spaces.
  ```text
  1	-	Review project architecture
  2	x	Buy fresh bread
  ```
- **JSON (`--json`)**: Single-line JSON with schema version and timestamps:
  ```json
  {"schema_version":1,"date":"2026-10-27","tasks":[{"index":1,"text":"Review project architecture","done":false,"done_at":null},{"index":2,"text":"Buy fresh bread","done":true,"done_at":"2026-10-27T11:53:00+02:00"}]}
  ```
  *(With `--all`, the output contains a `"days": [...]` array).*

#### `slate today`
- **Human (TTY)**: Prints the full weekday header, done/total counter, progress bar, and today's task list.
- **Plain (`--plain`)**: Exactly one line: `done/total` (e.g. `2/3`). If there are zero tasks, prints `0/0`. Ideal for status bars.
- **JSON (`--json`)**: Single-line JSON with completion percentages:
  ```json
  {"schema_version":1,"date":"2026-10-27","done":2,"total":3,"percent":66,"tasks":[...]}
  ```
- **Waybar (`--waybar`)**: Single JSON line designed for Waybar's custom JSON module:
  ```json
  {"text":"2/3","tooltip":"Review project architecture\nBuy fresh bread","class":"pending","percentage":66}
  ```
  *(The `class` attribute is `empty` when there are no tasks, `done` when all tasks are complete, or `pending`).*

---

### Exit Codes

Slate uses standard, predictable exit codes for scripting:

| Code | Meaning | Description |
| :---: | :--- | :--- |
| `0` | **Success** | Command completed successfully. |
| `1` | **Runtime / I/O Error** | Unreadable storage file, corrupt JSON database, lock acquisition timeout, or non-terminal environment without a subcommand. |
| `2` | **Usage Error** | Invalid date format, empty task text, missing arguments, or mutually exclusive options (e.g. `--plain` and `--json`). |
| `3` | **No Such Task** | Specified task index out of range, or `--match` found 0 matches or multiple ambiguous candidates. |

---

### Status Bar Integrations

#### 1. tmux
Display today's task counter in your tmux status bar:
```tmux
# ~/.tmux.conf
set -g status-right "#(slate today --plain) | %H:%M"
```

#### 2. Waybar
Add a custom module to your Waybar configuration:
```jsonc
// ~/.config/waybar/config
"custom/slate": {
    "exec": "slate today --waybar",
    "return-type": "json",
    "interval": 60,
    "format": "✓ {}",
    "on-click": "alacritty -e slate"
}
```

Style the module in `~/.config/waybar/style.css`:
```css
#custom-slate.pending {
    color: #e48c4b;
}

#custom-slate.done {
    color: #4ebaaf;
}

#custom-slate.empty {
    color: #888888;
}
```

#### 3. Polybar
Add a script module to Polybar:
```ini
; ~/.config/polybar/config.ini
[module/slate]
type = custom/script
exec = slate today --plain
interval = 60
format-prefix = "✓ "
click-left = alacritty -e slate
```

---

### Shell Completions

Generate shell completions for your shell using `slate completions <SHELL>`:

#### Bash
```bash
mkdir -p ~/.local/share/bash-completion/completions
slate completions bash > ~/.local/share/bash-completion/completions/slate
```

#### Zsh
```zsh
mkdir -p ~/.zfunc
slate completions zsh > ~/.zfunc/_slate

# Ensure ~/.zfunc is in your fpath by adding this to ~/.zshrc (before compinit):
# fpath=(~/.zfunc $fpath)
# autoload -Uz compinit && compinit
```

#### Fish
```fish
mkdir -p ~/.config/fish/completions
slate completions fish > ~/.config/fish/completions/slate.fish
```

#### PowerShell
```powershell
if (!(Test-Path -Path $PROFILE)) { New-Item -ItemType File -Path $PROFILE -Force }
slate completions powershell >> $PROFILE
```

---

### Export & Import

Slate allows exporting and importing your tasks across multiple formats (`json`, `md`, `csv`, `txt`). Format is automatically inferred from the file extension (`-o tasks.md` or `slate import tasks.csv`), or can be specified explicitly via `--format`.

| Format | Export Description | Import Description |
| :--- | :--- | :--- |
| **`json`** | Wrapped model `{"schema_version": 1, "days": [...]}` with ISO dates and RFC 3339 timestamps. Lossless round trip. | Validates schema version and task structure. Lossless round trip. |
| **`md`** | Per-day `## YYYY-MM-DD` headings, `- [x]` / `- [ ]` task items, and HTML comment timestamps `<!-- done RFC3339 -->`. | Parses `## YYYY-MM-DD` headers and `-`/`*` checkboxes (`[ ]`, `[x]`, `[X]`). Tasks before any heading default to `--date` (today). |
| **`csv`** | RFC 4180 quoting with header `date,text,done,done_at`. Quoted fields survive commas, quotes, and newlines. | Same columns. `done` accepts `true`, `false`, `1`, `0`, or `x`. Missing `done_at` defaults to `null`. |
| **`txt`** | Single day only (`--date`, default today). Tasks prefixed with `[x]` / `[ ]`. Multi-day export fails with hint to use `md` or `csv`. | Single day into `--date` (default today). Optional `[x]`/`[ ]` prefix. Blank lines and `#` comments ignored. |

#### Import Modes & Safety
- **All-or-Nothing Validation**: The entire file is parsed and validated before modifying storage. If any row is malformed, nothing is written and the exact line number and error reason are printed to `stderr`.
- **Merge Mode (Default)**: Appends tasks into their respective days while skipping exact duplicates (same date and same trimmed text, case-sensitive). Existing tasks are never modified or reordered.
- **Replace Mode (`--replace --yes`)**: Overwrites the entire task dataset. Requires the `--yes` confirmation flag. Automatically creates a safety backup of your existing data at `<datafile>.bak` before replacing.
- **Dry Run (`--dry-run`)**: Validates input and prints summary statistics without modifying storage (e.g. `would add 12, skip 3 duplicates`).
- **Encodings**: Transparently handles UTF-8 Byte Order Marks (BOM), Windows CRLF line endings, and empty files (treated as a successful no-op with a warning).

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
├── commands.rs     # CLI command handlers (add, done, undone, rm, list, today, path, completions)
├── exchange.rs     # Import and export engines (json, md, csv, txt) with all-or-nothing validation
├── output.rs       # Output formatters (human, plain, JSON, Waybar) and broken-pipe-safe writer
├── store.rs        # Shared domain operations on task collections (shared between TUI and CLI)
├── app.rs          # App state, task mutations, undo/redo stack, action dispatch, and mouse hit-testing
├── config.rs       # Runtime TOML configuration, strftime/color validation, and OS path resolution
├── keymap.rs       # Action enum, key chord parser, conflict detection, arrows mode, and dynamic hints
├── model.rs        # Data models (Todo, View, HistoryPane, InputMode)
├── storage/        # Storage subsystem:
│   ├── mod.rs      # Atomic file loading and saving, external change detection, try_load_modify_save
│   ├── migrate.rs  # Stepwise schema migrations (v0 -> v1) and version detection
│   ├── backup.rs   # Rolling backups engine, snapshot rotation, and pruning
│   └── lock.rs     # Sidecar advisory file locking (3-second timeout retry loop)
├── theme.rs        # Runtime Theme struct, presets (teal, orange, lavender, mono), and 256-color fallback
└── ui.rs           # Ratatui rendering, Symbols (Unicode/ASCII), HitArea tracking, and component drawers
```

### Module Responsibilities

1. **[`src/cli.rs`](src/cli.rs)**:
   Defines the command-line interface using `clap` derive, including global flags (`--data-file`, `--config`), subcommands, arguments, and after-help example blocks.
2. **[`src/commands.rs`](src/commands.rs)**:
   Implements command execution for `add`, `done`, `undone`, `rm`, `list`, `today`, `path`, and `completions`, including 1-based index validation, case-insensitive `--match` query resolution, and structured exit codes.
3. **[`src/output.rs`](src/output.rs)**:
   Generates human, plain tab-separated, single-line JSON, and Waybar formatted output. Implements `SafeWriter` to handle closed pipes (`EPIPE`) gracefully without panics.
4. **[`src/exchange.rs`](src/exchange.rs)**:
   Handles data exchange for JSON, Markdown, CSV, and plain text formats. Enforces all-or-nothing validation, duplicate skipping on merge, safety `.bak` creation on `--replace`, and handles UTF-8 BOM and Windows CRLF line endings.
5. **[`src/store.rs`](src/store.rs)**:
   Pure domain logic operating on `Vec<Todo>` (`add_task`, `set_task_done`, `delete_task`, `edit_task`, `clear_completed`, `reorder_day_task`, `match_task_by_text`). Shared between the interactive TUI and scriptable CLI.
6. **[`src/config.rs`](src/config.rs)**:
   Loads configuration from `config.toml` (or `SLATE_CONFIG` / `--config`), validates settings, and provides per-field fallback with non-blocking transient warnings on bad input.
7. **[`src/keymap.rs`](src/keymap.rs)**:
   Manages keymappings through the `Action` enum, parses key chord strings, enforces unbinding protection on `Ctrl+c`, detects conflicts, and generates dynamic hints.
8. **[`src/theme.rs`](src/theme.rs)**:
   Runtime `Theme` supporting presets (`teal`, `orange`, `lavender`, `mono`), custom accent color overrides, automatic ANSI-256 fallback, and `NO_COLOR` compliance.
9. **[`src/ui.rs`](src/ui.rs)**:
   Layout rendering, `Symbols` table (Unicode vs ASCII fallback), small terminal notice guard, and mouse hit-testing areas.
10. **[`src/storage/`](src/storage/)**:
    Provides crash-resilient data persistence:
    - **`storage/mod.rs`**: Resolves OS paths, atomic save engine (tempfiles, fsync, rename with retries), and locked load-modify-save helper.
    - **`storage/migrate.rs`**: Stepwise schema migrations from v0 formats.
    - **`storage/backup.rs`**: Rolling backup system creating timestamped snapshots in `backups/`.
    - **`storage/lock.rs`**: Advisory file locking via `todos.json.lock` with a 3-second timeout loop.
11. **[`src/app.rs`](src/app.rs)**:
    Maintains interactive TUI memory, undo/redo stacks, selection clamping, and routes input events.
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
| `ui.mouse` | boolean | `false` | When `true`, enables mouse interaction. When `false`, mouse capture is not enabled so terminal text selection remains uninterrupted. |
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
| `SLATE_DATA` | Overrides the default storage file path (overridden by `--data-file`). |

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
├── todos.json                       # Active task database (Schema version 1)
├── todos.json.lock                  # Advisory sidecar lock file (held only during saves)
├── todos.v0.bak                     # One-time legacy backup (created when migrating older format)
└── backups/                         # Rolling snapshots directory (up to 10 newest kept)
    ├── todos-20261027-143000.json
    └── todos-20261027-144500.json
```

### On-Disk Schema & Compatibility

Slate uses a versioned schema on disk:
```json
{
  "version": 1,
  "todos": [
    {
      "id": 1,
      "text": "Review project architecture",
      "created_date": "2026-10-27",
      "done": false,
      "done_at": null
    }
  ]
}
```

- **Backward Compatibility**: Legacy `todos.json` files (bare arrays `[...]` or unversioned objects) are detected as v0 and automatically migrated in memory to v1 on load.
- **Safety Backup (`todos.v0.bak`)**: The first time a legacy file is rewritten in the v1 format, a byte-for-byte copy of the original file is preserved as `todos.v0.bak` and never overwritten.
- **Future Versions**: If a `todos.json` was created by a newer version of Slate with a higher schema version, Slate refuses to load or overwrite the file, printing an upgrade message (`todos.json was written by a newer Slate (version X, this build supports Y). Upgrade Slate.`) and exiting cleanly without touching your data.
- **Syntax / Parse Errors**: If a `todos.json` file is corrupt, Slate prints the exact line, column, and parse error, reports the path to the newest backup in `backups/` if one exists, and exits non-zero without modifying the file.

### Concurrency & External Changes

- **Advisory Sidecar Locking**: Slate locks `todos.json.lock` only for the duration of a write operation, preventing simultaneous processes from corrupting data.
- **External Changes**: Slate does not live-reload changes if `todos.json` is modified externally while Slate is running. However, before saving, Slate checks the file's fingerprint (mtime + size). If the file changed on disk, Slate automatically creates a safety snapshot of the on-disk file in `backups/` before writing and displays a transient footer notice: `Data changed on disk · previous version backed up`.

