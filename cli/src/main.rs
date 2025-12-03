mod commands;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    ResolveExecute { tx_hash: String },
    SendIndex { destination_chain_id: u32 },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ResolveExecute { tx_hash } => {
            commands::resolve_execute(tx_hash)?;
        }
        Commands::SendIndex { destination_chain_id } => {
            commands::send_index(destination_chain_id)?;
        }
    }

    Ok(())
}
