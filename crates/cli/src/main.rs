mod deploy;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rune-cli")]
#[command(about = "Rune deployment CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Deploy {
        #[arg(long)]
        id: String,
        #[arg(long)]
        route: String,
        wasm: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy { id, route, wasm } => {
            let deployed = deploy::deploy_function(&id, &route, &wasm)?;

            println!(
                "deployed '{}' to route '{}' using artifact '{}'",
                deployed.id, deployed.route, deployed.wasm_path
            );
        }
    }

    Ok(())
}
