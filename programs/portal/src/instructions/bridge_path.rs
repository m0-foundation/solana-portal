use anchor_lang::prelude::*;
use common::BridgeError;

use crate::state::{BridgePath, ChainBridgePaths, PortalGlobal, CHAIN_PATHS_SEED, GLOBAL_SEED};

/// Initialize a ChainBridgePaths account for a destination chain
#[derive(Accounts)]
#[instruction(destination_chain_id: u32)]
pub struct InitializeChainPaths<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin @ BridgeError::NotAuthorized,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        init,
        payer = admin,
        space = ChainBridgePaths::size(0),
        seeds = [CHAIN_PATHS_SEED, destination_chain_id.to_be_bytes().as_ref()],
        bump,
    )]
    pub chain_paths: Account<'info, ChainBridgePaths>,

    pub system_program: Program<'info, System>,
}

impl InitializeChainPaths<'_> {
    pub fn handler(ctx: Context<Self>, destination_chain_id: u32) -> Result<()> {
        ctx.accounts.chain_paths.set_inner(ChainBridgePaths {
            bump: ctx.bumps.chain_paths,
            destination_chain_id,
            paths: Vec::new(),
        });

        emit!(ChainPathsInitialized { destination_chain_id });

        Ok(())
    }
}

#[event]
pub struct ChainPathsInitialized {
    pub destination_chain_id: u32,
}

/// Add a new bridge path to a chain's configuration
#[derive(Accounts)]
#[instruction(destination_chain_id: u32)]
pub struct AddBridgePath<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin @ BridgeError::NotAuthorized,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        mut,
        seeds = [CHAIN_PATHS_SEED, destination_chain_id.to_be_bytes().as_ref()],
        bump = chain_paths.bump,
        realloc = ChainBridgePaths::size(chain_paths.paths.len() + 1),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub chain_paths: Account<'info, ChainBridgePaths>,

    pub system_program: Program<'info, System>,
}

impl AddBridgePath<'_> {
    pub fn handler(
        ctx: Context<Self>,
        _destination_chain_id: u32,
        source_mint: Pubkey,
        destination_token: [u8; 32],
    ) -> Result<()> {
        let path = BridgePath {
            source_mint,
            destination_token,
        };

        // Check for duplicates
        require!(
            !ctx.accounts.chain_paths.paths.contains(&path),
            BridgeError::PathAlreadyExists
        );

        ctx.accounts.chain_paths.paths.push(path);

        emit!(BridgePathAdded {
            destination_chain_id: ctx.accounts.chain_paths.destination_chain_id,
            source_mint,
            destination_token,
        });

        Ok(())
    }
}

#[event]
pub struct BridgePathAdded {
    pub destination_chain_id: u32,
    pub source_mint: Pubkey,
    pub destination_token: [u8; 32],
}

/// Remove a bridge path from a chain's configuration
#[derive(Accounts)]
#[instruction(destination_chain_id: u32, source_mint: Pubkey, destination_token: [u8; 32])]
pub struct RemoveBridgePath<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin @ BridgeError::NotAuthorized,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        mut,
        seeds = [CHAIN_PATHS_SEED, destination_chain_id.to_be_bytes().as_ref()],
        bump = chain_paths.bump,
        realloc = ChainBridgePaths::size(chain_paths.paths.len().saturating_sub(1)),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub chain_paths: Account<'info, ChainBridgePaths>,

    pub system_program: Program<'info, System>,
}

impl RemoveBridgePath<'_> {
    pub fn handler(
        ctx: Context<Self>,
        _destination_chain_id: u32,
        source_mint: Pubkey,
        destination_token: [u8; 32],
    ) -> Result<()> {
        let paths = &mut ctx.accounts.chain_paths.paths;
        let initial_len = paths.len();

        paths.retain(|p| !(p.source_mint == source_mint && p.destination_token == destination_token));

        require!(paths.len() < initial_len, BridgeError::PathNotFound);

        emit!(BridgePathRemoved {
            destination_chain_id: ctx.accounts.chain_paths.destination_chain_id,
            source_mint,
            destination_token,
        });

        Ok(())
    }
}

#[event]
pub struct BridgePathRemoved {
    pub destination_chain_id: u32,
    pub source_mint: Pubkey,
    pub destination_token: [u8; 32],
}
