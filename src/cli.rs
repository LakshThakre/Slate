use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "slate",
    version,
    about = "A minimal terminal todo app",
    propagate_version = true
)]
pub struct Cli {
    #[arg(
        long = "data-file",
        global = true,
        value_name = "PATH",
        help = "Path to storage file"
    )]
    pub data_file: Option<PathBuf>,

    #[arg(
        long = "config",
        global = true,
        value_name = "PATH",
        help = "Path to config file"
    )]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    #[command(
        about = "Add one or more tasks",
        after_help = "Examples:\n  slate add Buy milk\n  slate add \"Plan architecture\" \"Review PR\"\n  cat tasks.txt | slate add -\n  slate add --date tomorrow \"Call dentist\"\n  slate add -q \"Silent task\""
    )]
    Add {
        #[arg(
            value_name = "TEXT",
            required = true,
            num_args = 1..,
            help = "Task text to add (or - to read line-by-line from stdin)"
        )]
        text: Vec<String>,

        #[arg(
            long,
            value_name = "DATE",
            help = "Target date (today, yesterday, tomorrow, or YYYY-MM-DD)"
        )]
        date: Option<String>,

        #[arg(short = 'q', long, help = "Do not print confirmation")]
        quiet: bool,
    },

    #[command(
        about = "Mark tasks done",
        after_help = "Examples:\n  slate done 1\n  slate done 1 2 3\n  slate done --match \"dentist\"\n  slate done --date yesterday 1"
    )]
    Done {
        #[arg(value_name = "N", help = "1-based task indices")]
        indices: Vec<usize>,

        #[arg(
            long,
            value_name = "DATE",
            help = "Target date (today, yesterday, tomorrow, or YYYY-MM-DD)"
        )]
        date: Option<String>,

        #[arg(
            long = "match",
            value_name = "TEXT",
            help = "Case-insensitive substring to match a single task"
        )]
        match_query: Option<String>,
    },

    #[command(
        about = "Mark tasks unfinished",
        after_help = "Examples:\n  slate undone 1\n  slate undone 1 2\n  slate undone --match \"milk\"\n  slate undone --date yesterday 2"
    )]
    Undone {
        #[arg(value_name = "N", help = "1-based task indices")]
        indices: Vec<usize>,

        #[arg(
            long,
            value_name = "DATE",
            help = "Target date (today, yesterday, tomorrow, or YYYY-MM-DD)"
        )]
        date: Option<String>,

        #[arg(
            long = "match",
            value_name = "TEXT",
            help = "Case-insensitive substring to match a single task"
        )]
        match_query: Option<String>,
    },

    #[command(
        about = "Delete tasks",
        after_help = "Examples:\n  slate rm 1\n  slate rm 1 2 3\n  slate rm --match \"temporary task\""
    )]
    Rm {
        #[arg(value_name = "N", help = "1-based task indices")]
        indices: Vec<usize>,

        #[arg(
            long,
            value_name = "DATE",
            help = "Target date (today, yesterday, tomorrow, or YYYY-MM-DD)"
        )]
        date: Option<String>,

        #[arg(
            long = "match",
            value_name = "TEXT",
            help = "Case-insensitive substring to match a single task"
        )]
        match_query: Option<String>,
    },

    #[command(
        about = "List tasks",
        after_help = "Examples:\n  slate list\n  slate list --date tomorrow\n  slate list --all --pending\n  slate list --limit 5\n  slate list --plain\n  slate list --json"
    )]
    List {
        #[arg(
            long,
            value_name = "DATE",
            help = "Target date (today, yesterday, tomorrow, or YYYY-MM-DD)"
        )]
        date: Option<String>,

        #[arg(long, help = "Show tasks across all days (newest first)")]
        all: bool,

        #[arg(long, help = "Show unfinished tasks only")]
        pending: bool,

        #[arg(long, value_name = "N", help = "Limit number of tasks displayed")]
        limit: Option<usize>,

        #[arg(long, help = "Plain tab-separated output")]
        plain: bool,

        #[arg(long, help = "JSON output")]
        json: bool,
    },

    #[command(
        about = "Summary for today (weekday header, progress, tasks)",
        after_help = "Examples:\n  slate today\n  slate today --plain\n  slate today --json\n  slate today --waybar"
    )]
    Today {
        #[arg(long, help = "Plain text output: done/total (e.g. 2/3)")]
        plain: bool,

        #[arg(long, help = "JSON output with stats and tasks")]
        json: bool,

        #[arg(long, help = "Single JSON line formatted for Waybar custom module")]
        waybar: bool,
    },

    #[command(
        about = "Print the resolved storage data path",
        after_help = "Examples:\n  slate path\n  slate --data-file /tmp/test.json path"
    )]
    Path,

    #[command(
        about = "Generate shell completions",
        after_help = "Examples:\n  slate completions bash > ~/.local/share/bash-completion/completions/slate\n  slate completions zsh > ~/.zfunc/_slate\n  slate completions fish > ~/.config/fish/completions/slate.fish\n  slate completions powershell >> $PROFILE"
    )]
    Completions {
        #[arg(value_name = "SHELL", help = "Shell to generate completions for")]
        shell: clap_complete::Shell,
    },
}
