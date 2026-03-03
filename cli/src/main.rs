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

#[derive(Clone, Copy, ValueEnum, Debug, PartialEq, Eq)]
pub enum Network {
    Devnet,
    Mainnet,
    Testnet,
}

#[derive(Subcommand)]
enum Commands {
    SendIndex {
        destination_chain_id: u32,
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
        #[arg(short, long, value_enum, env = "NETWORK", default_value = "devnet")]
        network: Network,
    },
    SendEvmIndex {
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
        #[arg(short, long, value_enum, env = "NETWORK", default_value = "devnet")]
        network: Network,
    },
    SendToken {
        amount: u64,
        destination_chain_id: u32,
        recipient: String,
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
        #[arg(short, long, value_enum, env = "NETWORK", default_value = "devnet")]
        network: Network,
    },
    SendEvmToken {
        amount: u128,
        recipient: String,
        #[arg(short, long, value_enum, default_value = "hyperlane")]
        adapter: BridgeAdapter,
        #[arg(short, long, value_enum, env = "NETWORK", default_value = "devnet")]
        network: Network,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::SendIndex {
            destination_chain_id,
            adapter,
            network,
        } => {
            commands::send_index(destination_chain_id, adapter, network).await?;
        }
        Commands::SendEvmIndex { adapter, network } => {
            commands::send_evm_index(adapter, network).await?;
        }
        Commands::SendToken {
            amount,
            destination_chain_id,
            recipient,
            adapter,
            network,
        } => {
            commands::send_token(amount, destination_chain_id, recipient, adapter, network).await?;
        }
        Commands::SendEvmToken {
            amount,
            recipient,
            adapter,
            network,
        } => {
            commands::send_evm_token(amount, recipient, adapter, network).await?;
        }
    }

    Ok(())
}
