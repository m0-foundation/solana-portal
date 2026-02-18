use anchor_lang::prelude::*;
use m0_portal_common::BridgeError;
use std::collections::HashSet;

use crate::state::{BridgePath, ChainBridgePaths, PortalGlobal, CHAIN_PATHS_SEED, GLOBAL_SEED};

#[derive(Accounts)]
#[instruction(destination_chain_id: u32)]
pub struct InitializeBridgePaths<'info> {
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
        seeds = [CHAIN_PATHS_SEED, &destination_chain_id.to_be_bytes()],
        bump,
    )]
    pub chain_paths: Account<'info, ChainBridgePaths>,

    pub system_program: Program<'info, System>,
}

impl InitializeBridgePaths<'_> {
    pub fn handler(ctx: Context<Self>, destination_chain_id: u32) -> Result<()> {
        ctx.accounts.chain_paths.set_inner(ChainBridgePaths {
            bump: ctx.bumps.chain_paths,
            destination_chain_id,
            paths: Vec::new(),
        });

        emit!(ChainPathsInitialized {
            destination_chain_id
        });

        Ok(())
    }
}

#[event]
pub struct ChainPathsInitialized {
    pub destination_chain_id: u32,
}

#[derive(Accounts)]
#[instruction(destination_chain_id: u32, paths: Vec<BridgePath>)]
pub struct AddBridgePaths<'info> {
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
        seeds = [CHAIN_PATHS_SEED, &destination_chain_id.to_be_bytes()],
        bump = chain_paths.bump,
        realloc = ChainBridgePaths::size(chain_paths.paths.len() + paths.len()),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub chain_paths: Account<'info, ChainBridgePaths>,

    pub system_program: Program<'info, System>,
}

impl AddBridgePaths<'_> {
    fn validate(&self, paths: &Vec<BridgePath>) -> Result<()> {
        // List must not contain duplicates
        let mut seen = HashSet::new();
        let all_unique = paths.iter().all(|p| seen.insert(p));
        require!(all_unique, BridgeError::DuplicatePath);

        for p in paths {
            // Paths must not already exist in the current list
            require!(
                !self.chain_paths.paths.contains(p),
                BridgeError::PathAlreadyExists
            );

            require!(
                p.source_mint != Pubkey::default() && p.destination_token != [0u8; 32],
                BridgeError::InvalidPath
            );
        }

        Ok(())
    }

    #[access_control(ctx.accounts.validate(&paths))]
    pub fn handler(
        ctx: Context<Self>,
        destination_chain_id: u32,
        paths: Vec<BridgePath>,
    ) -> Result<()> {
        for path in paths {
            ctx.accounts.chain_paths.paths.push(path.clone());

            emit!(BridgePathAdded {
                destination_chain_id,
                source_mint: path.source_mint,
                destination_token: path.destination_token,
            });
        }

        Ok(())
    }
}

#[event]
pub struct BridgePathAdded {
    pub destination_chain_id: u32,
    pub source_mint: Pubkey,
    pub destination_token: [u8; 32],
}

#[derive(Accounts)]
#[instruction(destination_chain_id: u32)]
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
        seeds = [CHAIN_PATHS_SEED, &destination_chain_id.to_be_bytes()],
        bump = chain_paths.bump,
        realloc = ChainBridgePaths::size(chain_paths.paths.len().saturating_sub(1)),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub chain_paths: Account<'info, ChainBridgePaths>,

    pub system_program: Program<'info, System>,
}

impl RemoveBridgePath<'_> {
    fn validate(&self, path: &BridgePath) -> Result<()> {
        require!(
            self.chain_paths.paths.contains(path),
            BridgeError::PathNotFound
        );

        Ok(())
    }

    #[access_control(ctx.accounts.validate(&path))]
    pub fn handler(ctx: Context<Self>, destination_chain_id: u32, path: BridgePath) -> Result<()> {
        ctx.accounts.chain_paths.paths.retain(|p| {
            p.source_mint != path.source_mint || p.destination_token != path.destination_token
        });

        emit!(BridgePathRemoved {
            destination_chain_id,
            source_mint: path.source_mint,
            destination_token: path.destination_token,
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
