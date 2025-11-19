use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "codex-view-image")]
#[command(about = "Load and display information about an image file", long_about = None)]
struct Cli {
    /// Path to the image file
    #[arg(value_name = "IMAGE")]
    path: PathBuf,

    /// Output results as JSON
    #[arg(long)]
    json: bool,

    /// Show base64-encoded image data
    #[arg(long)]
    show_data: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let path = if cli.path.is_absolute() {
        cli.path
    } else {
        std::env::current_dir()?.join(cli.path)
    };

    let result = codex_view_image::view_image(&path).await?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Path: {}", result.path);
        println!("MIME Type: {}", result.mime_type);
        println!("Dimensions: {}x{}", result.width, result.height);
        if cli.show_data {
            println!("Base64 Data: {}", result.base64_data);
        } else {
            println!(
                "Base64 Data: {} bytes (use --show-data to display)",
                result.base64_data.len()
            );
        }
    }

    Ok(())
}
