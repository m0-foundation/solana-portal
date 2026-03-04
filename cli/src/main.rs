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
    SendIndex {
        destination_chain_id: u32,
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
    },
    SendEvmIndex {
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
    },
    SendToken {
        amount: u64,
        destination_chain_id: u32,
        recipient: String,
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
    },
    SendEvmToken {
        amount: u128,
        recipient: String,
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
    },
    CreateHyperlaneLut {
        #[arg(long, value_parser = ["mainnet", "testnet"])]
        network: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::SendIndex {
            destination_chain_id,
            adapter,
        } => {
            commands::send_index(destination_chain_id, adapter).await?;
        }
        Commands::SendEvmIndex { adapter } => {
            commands::send_evm_index(adapter).await?;
        }
        Commands::SendToken {
            amount,
            destination_chain_id,
            recipient,
            adapter,
        } => {
            commands::send_token(amount, destination_chain_id, recipient, adapter).await?;
        }
        Commands::SendEvmToken {
            amount,
            recipient,
            adapter,
        } => {
            commands::send_evm_token(amount, recipient, adapter).await?;
        }
        Commands::CreateHyperlaneLut { network } => {
            commands::create_hyperlane_lut(network).await?;
        }
    }

    Ok(())
}
