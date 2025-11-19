use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "codex-grep-files")]
#[command(about = "Search for files containing a pattern using ripgrep", long_about = None)]
struct Cli {
    /// Regular expression pattern to search for
    #[arg(value_name = "PATTERN")]
    pattern: String,

    /// Optional glob to filter which files are searched (e.g., "*.rs")
    #[arg(short, long, value_name = "GLOB")]
    include: Option<String>,

    /// Path to search in (defaults to current directory)
    #[arg(short, long, value_name = "PATH")]
    path: Option<PathBuf>,

    /// Maximum number of results to return (default: 100, max: 2000)
    #[arg(short, long, default_value = "100")]
    limit: usize,

    /// Output results as JSON
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let search_path = cli.path.as_deref().unwrap_or(std::path::Path::new("."));
    let cwd = std::env::current_dir()?;

    let results = codex_grep_files::grep_files(
        &cli.pattern,
        cli.include.as_deref(),
        search_path,
        cli.limit,
        &cwd,
    )
    .await?;

    if results.is_empty() {
        if cli.json {
            println!("[]");
        } else {
            eprintln!("No matches found.");
        }
    } else if cli.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for result in results {
            println!("{result}");
        }
    }

    Ok(())
}
