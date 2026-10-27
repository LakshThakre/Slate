# Slate

[![CI](https://github.com/LakshThakre/Slate/actions/workflows/ci.yml/badge.svg)](https://github.com/LakshThakre/Slate/actions/workflows/ci.yml)

> A minimal, calm, and distraction-free terminal todo app built in Rust.

Slate is designed in the spirit of modern, quiet developer tools like Claude Code and Codex CLI. It discards heavy boxed borders in favor of generous whitespace, subtle padding, thin divider rules, and a restrained color palette that respects your terminal's natural theme.

---

## Features

- **Quiet, Minimal Interface**: Clean layout without heavy box borders, using a soft teal accent and breathable spacing.
- **Cross-Platform**: Works smoothly across Linux, macOS, and Windows.
- **Today & History Views**:
  - **Today View**: See today's tasks with a friendly weekday header and a slim block progress bar (`████░░░░░░`). Shows subtle overdue and carry-over indicators.
  - **History View**: Browse past days by date (`Today`, `Yesterday`, `Mon 19 Oct`) with task counts, completion timestamps (`done 14:32`), and overdue indicators.
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
- **Lightweight & Fast**: Sub-millisecond startup time, negligible memory footprint, and low CPU usage.

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

## Codebase Architecture

Slate is organized into small, focused modules under `src/`:

```text
src/
├── main.rs         # Application lifecycle, panic hook, terminal initialization, event loop with midnight timeout
├── app.rs          # App state struct, task mutations, undo/redo stack, and key handlers
├── model.rs        # Data models (Todo, View, HistoryPane, InputMode)
├── storage/        # Storage subsystem:
│   ├── mod.rs      # Atomic file loading and saving, external change detection
│   ├── migrate.rs  # Stepwise schema migrations (v0 -> v1) and version detection
│   ├── backup.rs   # Rolling backups engine, snapshot rotation, and pruning
│   └── lock.rs     # Sidecar advisory file locking (3-second timeout retry loop)
├── theme.rs        # Theme struct and color palette configuration
└── ui.rs           # Ratatui layout rendering, small terminal guard, and component drawing helpers
```

### Module Responsibilities

1. **[`src/theme.rs`](src/theme.rs)**:
   Defines the `THEME` palette with restrained tones: teal `accent`, soft orange `warn` for overdue states, light red `error`, and subtle dividers.
2. **[`src/model.rs`](src/model.rs)**:
   Declares the core `Todo` struct (with serde defaults for full backward compatibility) along with `View`, `HistoryPane`, and `InputMode` enums.
3. **[`src/storage/`](src/storage/)**:
   Provides robust, crash-resilient data persistence:
   - **`storage/mod.rs`**: Resolves OS paths, atomic save engine (PID-tagged tempfiles, fsync, rename with retries, parent sync on Unix), and external change detection.
   - **`storage/migrate.rs`**: Stepwise schema migrations. Detects v0 legacy formats (unversioned arrays or objects) and migrates them safely in memory.
   - **`storage/backup.rs`**: Rolling backup system taking timestamped snapshots (`todos-YYYYMMDD-HHMMSS.json`), pruning old snapshots to the 10 newest, and deduplicating identical snapshots.
   - **`storage/lock.rs`**: Advisory file locking via a sidecar lock file (`todos.json.lock`), held only during saves or atomic load-modify-save blocks.
4. **[`src/app.rs`](src/app.rs)**:
   Maintains application memory, implements the centralized snapshot-based mutation engine (`mutate`) powering instant undo/redo, manages selection clamping, daily rollover tracking, and routes keyboard events.
5. **[`src/ui.rs`](src/ui.rs)**:
   Handles all screen painting with dedicated helpers:
   - `draw_header`: Top brand title, navigation tabs, and overdue counter.
   - `draw_today`: Title row, block progress bar, carry-over hint, and today's task rows.
   - `draw_history`: Responsive date column, vertical line rule (`│`), and date task pane with completion timestamps and overdue markers.
   - `draw_help`: Minimal centered overlay displaying grouped keyboard shortcuts.
   - `draw_input`: Prompt line with cursor placement for adding and editing.
   - `draw_footer`: Compact context-aware key hints and transient status messages.
6. **[`src/main.rs`](src/main.rs)**:
   Lightweight orchestrator that initializes the terminal in raw mode, installs a panic hook for safe recovery, computes local midnight timeouts for zero-CPU idle polling, and restores the terminal on exit.

---

## Theming & Customization

To change colors, open [`src/theme.rs`](src/theme.rs) and edit the `THEME` constant:

```rust
pub const THEME: Theme = Theme {
    // Soft Teal (Default):
    accent: Color::Rgb(78, 186, 175),

    text: Color::Reset,
    dim: Color::DarkGray,
    subtext: Color::Gray,
    divider: Color::DarkGray,
    error: Color::LightRed,
    warn: Color::Rgb(228, 140, 75), // Soft orange warning for overdue
};
```

After modifying, recompile and install with:

```bash
cargo install --path .
```

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

