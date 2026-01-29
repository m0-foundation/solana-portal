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
    ResolveExecute {
        tx_hash: String,
    },
    SendIndex {
        destination_chain_id: u32,
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
    },
    SendEvmIndex {
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
    },
    ProcessHyperlaneMessage {
        message_id_hex: String,
        raw_hex: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ResolveExecute { tx_hash } => {
            commands::resolve_execute(tx_hash).await?;
        }
        Commands::SendIndex {
            destination_chain_id,
            adapter,
        } => {
            commands::send_index(destination_chain_id, adapter)?;
        }
        Commands::SendEvmIndex { adapter } => {
            commands::send_evm_index(adapter).await?;
        }
        Commands::ProcessHyperlaneMessage {
            message_id_hex,
            raw_hex,
        } => {
            commands::process_hyperlane_message(message_id_hex, raw_hex)?;
        }
    }

    Ok(())
}
