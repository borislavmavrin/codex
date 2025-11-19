use clap::Parser;
use clap::ValueEnum;
use std::path::PathBuf;

#[derive(Clone, ValueEnum)]
enum Mode {
    Slice,
    Indentation,
}

#[derive(Parser)]
#[command(name = "codex-read-file")]
#[command(about = "Read files with support for line ranges and indentation-aware blocks", long_about = None)]
struct Cli {
    /// File path to read
    #[arg(value_name = "FILE")]
    file_path: PathBuf,

    /// Starting line number (1-indexed)
    #[arg(short, long, default_value = "1")]
    offset: usize,

    /// Maximum number of lines to return
    #[arg(short, long, default_value = "2000")]
    limit: usize,

    /// Read mode: slice or indentation
    #[arg(short, long, default_value = "slice")]
    mode: Mode,

    /// Anchor line for indentation mode (defaults to offset)
    #[arg(long)]
    anchor_line: Option<usize>,

    /// Max indentation levels for indentation mode (0 = unlimited)
    #[arg(long, default_value = "0")]
    max_levels: usize,

    /// Include siblings at same indentation level
    #[arg(long)]
    include_siblings: bool,

    /// Include header comments above anchor block
    #[arg(long, default_value = "true")]
    include_header: bool,

    /// Hard cap on lines for indentation mode
    #[arg(long)]
    max_lines: Option<usize>,

    /// Output results as JSON
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let file_path = if cli.file_path.is_absolute() {
        cli.file_path
    } else {
        std::env::current_dir()?.join(cli.file_path)
    };

    let mode = match cli.mode {
        Mode::Slice => codex_read_file::ReadMode::Slice,
        Mode::Indentation => codex_read_file::ReadMode::Indentation,
    };

    let indentation = if matches!(mode, codex_read_file::ReadMode::Indentation) {
        Some(codex_read_file::IndentationArgs {
            anchor_line: cli.anchor_line,
            max_levels: cli.max_levels,
            include_siblings: cli.include_siblings,
            include_header: cli.include_header,
            max_lines: cli.max_lines,
        })
    } else {
        None
    };

    let results =
        codex_read_file::read_file(&file_path, cli.offset, cli.limit, mode, indentation).await?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for result in results {
            println!("{result}");
        }
    }

    Ok(())
}
