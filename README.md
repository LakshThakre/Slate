# Slate

> A minimal, calm, and distraction-free terminal todo app built in Rust.

Slate is designed in the spirit of modern, quiet developer tools like Claude Code and Codex CLI. It discards heavy boxed borders in favor of generous whitespace, subtle padding, thin divider rules, and a restrained color palette that respects your terminal's natural theme.

---

## Features

- **Quiet, Minimal Interface**: Clean layout without heavy box borders, using a soft teal accent and breathable spacing.
- **Cross-Platform**: Works smoothly across Linux, macOS, and Windows.
- **Today & History Views**:
  - **Today View**: See today's tasks with a friendly weekday header and a slim block progress bar (`████░░░░░░`).
  - **History View**: Browse past days by date (`Today`, `Yesterday`, `Mon 19 Oct`) with task counts and completion timestamps (`done 14:32`).
- **Prompt-Style Input**: Tap `a` to open a modern inline CLI prompt right at the bottom of the screen (`› What needs doing today?`).
- **Crash-Proof Atomic Storage**: Saves automatically after every change using a safe write-to-temp-and-rename pattern, preventing file corruption.
- **Single Plain-JSON Storage**: Stores tasks in your operating system's standard application data directory as human-readable JSON.
- **Lightweight & Fast**: Sub-millisecond startup time, negligible memory footprint, and low CPU usage.

---

## Visual Preview

### Today View
```text
  Slate    Today   History
  ──────────────────────────────────────────────────────────────────────────

  Friday, 23 October    2 of 3 done  ██████░░░░

  › ○ Review project architecture
    ✓ Buy fresh bread
    ✓ Read Rust documentation

  a add  ·  space done  ·  d delete  ·  tab history  ·  q quit
```

### Inline Prompt (while typing `a`)
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
    Yesterday       4/4 │
    Wed 21 Oct      1/2 │  › ○ Review project architecture
                        │    ✓ Buy fresh bread                    done 11:53
                        │    ✓ Read Rust documentation            done 12:10

  enter/l todos  ·  j/k navigate  ·  a add  ·  tab today  ·  q quit
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
| `q` | Quit Slate |
| `a` | Add a new task (opens the inline prompt) |

### Today View
| Key | Action |
| :--- | :--- |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Space` / `Enter` | Toggle task completion (`○` ↔ `✓`) |
| `d` / `Delete` | Delete the selected task |

### History View
| Key | Action |
| :--- | :--- |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` / `l` / `→` | Switch focus from Dates column to Tasks column |
| `h` / `←` | Switch focus from Tasks column back to Dates column |
| `Space` / `Enter` | Toggle task completion (in Tasks column) |
| `d` / `Delete` | Delete task (in Tasks column) |

### Inline Input Prompt (Adding Mode)
| Key | Action |
| :--- | :--- |
| `Enter` | Save task and return to Normal mode |
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
├── app.rs       # App state struct, task mutations, selection clamping, and key handlers
├── model.rs     # Data models (Todo, View, HistoryPane, InputMode)
├── storage.rs   # Atomic file loading and saving
├── theme.rs     # Theme struct and color palette configuration
└── ui.rs        # Ratatui layout rendering and visual component drawing helpers
```

### Module Responsibilities

1. **[`src/theme.rs`](file:///home/laksh/Projects/Todo/src/theme.rs)**:
   Defines the `THEME` palette. You can customize the accent color (e.g. from Soft Teal to Soft Orange or Soft Lavender) in a single line.
2. **[`src/model.rs`](file:///home/laksh/Projects/Todo/src/model.rs)**:
   Declares the core `Todo` struct along with `View`, `HistoryPane`, and `InputMode` enums.
3. **[`src/storage.rs`](file:///home/laksh/Projects/Todo/src/storage.rs)**:
   Resolves the standard storage path for each OS, deserializes JSON on load, and performs safe atomic saves by writing to a hidden temporary file before renaming.
4. **[`src/app.rs`](file:///home/laksh/Projects/Todo/src/app.rs)**:
   Maintains application memory, implements task manipulation (`add_todo`, `toggle_*`, `delete_*`), bounds-checked selection clamping (`clamp_selections`), and keyboard routing.
5. **[`src/ui.rs`](file:///home/laksh/Projects/Todo/src/ui.rs)**:
   Handles all screen painting with dedicated helpers:
   - `draw_header`: Top brand title and navigation tabs.
   - `draw_today`: Title row, block progress bar, and today's task rows.
   - `draw_history`: Narrow date column, vertical line rule (`│`), and date task pane with completion timestamps.
   - `draw_input`: Prompt line with cursor placement.
   - `draw_footer`: Context-aware key hints.
6. **[`src/main.rs`](file:///home/laksh/Projects/Todo/src/main.rs)**:
   Lightweight orchestrator that initializes the terminal in raw mode, installs a panic hook for safe recovery, runs the waiter-style event loop, and restores the terminal on exit.

---

## Theming & Customization

To change the accent color, open [`src/theme.rs`](file:///home/laksh/Projects/Todo/src/theme.rs) and edit the `accent` field:

```rust
pub const THEME: Theme = Theme {
    // Soft Teal (Default):
    accent: Color::Rgb(78, 186, 175),

    // Or try Soft Orange:
    // accent: Color::Rgb(228, 140, 75),

    // Or try Soft Lavender:
    // accent: Color::Rgb(180, 140, 220),

    text: Color::Reset,
    dim: Color::DarkGray,
    subtext: Color::Gray,
    divider: Color::DarkGray,
    error: Color::LightRed,
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
