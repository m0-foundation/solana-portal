mod commands;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum, Debug)]
pub enum BridgeAdapter {
    Hyperlane,
    Wormhole,
}

#[derive(Subcommand)]
enum Commands {
    ResolveExecute { tx_hash: String },
    SendIndex {
        destination_chain_id: u32,
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ResolveExecute { tx_hash } => {
            commands::resolve_execute(tx_hash)?;
        }
        Commands::SendIndex {
            destination_chain_id,
            adapter,
        } => {
            commands::send_index(destination_chain_id, adapter)?;
        }
    }

    Ok(())
}
