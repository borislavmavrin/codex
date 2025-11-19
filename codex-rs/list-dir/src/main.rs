use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "codex-list-dir")]
#[command(about = "List directory contents recursively", long_about = None)]
struct Cli {
    /// Directory path to list
    #[arg(value_name = "DIR")]
    dir_path: PathBuf,

    /// Starting entry number (1-indexed)
    #[arg(short, long, default_value = "1")]
    offset: usize,

    /// Maximum number of entries to return
    #[arg(short, long, default_value = "25")]
    limit: usize,

    /// Maximum directory depth to traverse
    #[arg(short, long, default_value = "2")]
    depth: usize,

    /// Output results as JSON
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let dir_path = if cli.dir_path.is_absolute() {
        cli.dir_path
    } else {
        std::env::current_dir()?.join(cli.dir_path)
    };

    let results = codex_list_dir::list_dir(&dir_path, cli.offset, cli.limit, cli.depth).await?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for result in results {
            println!("{result}");
        }
    }

    Ok(())
}
