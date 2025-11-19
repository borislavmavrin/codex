use clap::Parser;
use codex_protocol::plan_tool::UpdatePlanArgs;

#[derive(Parser)]
#[command(name = "codex-plan")]
#[command(about = "Process and validate agent task plans", long_about = None)]
struct Cli {
    /// JSON string containing the plan update
    #[arg(value_name = "JSON")]
    json: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let args: UpdatePlanArgs = serde_json::from_str(&cli.json)
        .map_err(|e| anyhow::anyhow!("failed to parse JSON: {e}"))?;

    let output = codex_plan::update_plan(args)?;

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
