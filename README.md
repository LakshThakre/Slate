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
- **Crash-Proof Atomic Storage**: Saves automatically after every change using a safe write-to-temp-and-rename pattern, preventing file corruption.
- **Single Plain-JSON Storage**: Stores tasks in your operating system's standard application data directory as human-readable JSON. Old files load seamlessly.
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
├── main.rs      # Application lifecycle, panic hook, terminal initialization, event loop
├── app.rs       # App state struct, task mutations, undo/redo stack, and key handlers
├── model.rs     # Data models (Todo, View, HistoryPane, InputMode)
├── storage.rs   # Atomic file loading and saving, backward compatibility
├── theme.rs     # Theme struct and color palette configuration
└── ui.rs        # Ratatui layout rendering, help overlay, and component drawing helpers
```

### Module Responsibilities

1. **[`src/theme.rs`](src/theme.rs)**:
   Defines the `THEME` palette with restrained tones: teal `accent`, soft orange `warn` for overdue states, light red `error`, and subtle dividers.
2. **[`src/model.rs`](src/model.rs)**:
   Declares the core `Todo` struct (with serde defaults for full backward compatibility) along with `View`, `HistoryPane`, and `InputMode` enums.
3. **[`src/storage.rs`](src/storage.rs)**:
   Resolves the standard storage path for each OS, deserializes legacy and modern JSON formats seamlessly, and performs safe atomic saves by writing to a hidden temporary file before renaming.
4. **[`src/app.rs`](src/app.rs)**:
   Maintains application memory, implements the centralized snapshot-based mutation engine (`mutate`) powering instant undo/redo, manages selection clamping, and routes keyboard events.
5. **[`src/ui.rs`](src/ui.rs)**:
   Handles all screen painting with dedicated helpers:
   - `draw_header`: Top brand title, navigation tabs, and overdue counter.
   - `draw_today`: Title row, block progress bar, carry-over hint, and today's task rows.
   - `draw_history`: Responsive date column, vertical line rule (`│`), and date task pane with completion timestamps and overdue markers.
   - `draw_help`: Minimal centered overlay displaying grouped keyboard shortcuts.
   - `draw_input`: Prompt line with cursor placement for adding and editing.
   - `draw_footer`: Compact context-aware key hints and transient status messages.
6. **[`src/main.rs`](src/main.rs)**:
   Lightweight orchestrator that initializes the terminal in raw mode, installs a panic hook for safe recovery, runs the waiter-style event loop, and restores the terminal on exit.

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

Because it is plain JSON, you can inspect it with any text editor, format it with `jq`, or back it up with your preferred tools at any time.
