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
    /// Manually relay a Wormhole VAA message by fetching it from WormholeScan and submitting on-chain
    RelayMessage {
        /// VAA ID in format: chain/emitter/sequence
        vaa_id: String,
        /// RPC URL override (defaults to RPC_URL env var)
        #[arg(long)]
        rpc_url: Option<String>,
        /// Use testnet WormholeScan API instead of mainnet
        #[arg(long)]
        testnet: bool,
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
        Commands::RelayMessage {
            vaa_id,
            rpc_url,
            testnet,
        } => {
            commands::relay_message(vaa_id, rpc_url, testnet).await?;
        }
    }

    Ok(())
}
