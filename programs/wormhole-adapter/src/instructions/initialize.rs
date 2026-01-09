use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{spl_token_2022::instruction::AuthorityType, Token2022},
    token_interface::{self, Mint},
};
use common::{portal, Peers, AUTHORITY_SEED};

use crate::state::{WormholeGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space =  WormholeGlobal::size(0),
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,

    #[account(
        mut,
        mint::token_program = token_program,
    )]
    pub m_mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [b"token_authority"],
        bump,
    )]
    /// CHECK: authority validated by seeds
    pub old_token_authority: UncheckedAccount<'info>,

    #[account(
        seeds = [AUTHORITY_SEED],
        seeds::program = portal::ID,
        bump,
    )]
    /// CHECK: authority validated by seeds
    pub new_token_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}

impl Initialize<'_> {
    pub fn handler(ctx: Context<Self>, chain_id: u32) -> Result<()> {
        ctx.accounts.wormhole_global.set_inner(WormholeGlobal {
            bump: ctx.bumps.wormhole_global,
            admin: ctx.accounts.admin.key(),
            outgoing_paused: false,
            incoming_paused: false,
            chain_id,
            peers: Peers::default(),
            pending_admin: None,
            receive_lut: None,
            padding: [0u8; 128],
        });

        // Relinquish mint authority
        // Previously, Wormhole was the only bridge and minted tokens
        if ctx.accounts.m_mint.mint_authority.unwrap() == ctx.accounts.old_token_authority.key() {
            token_interface::set_authority(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token_interface::SetAuthority {
                        account_or_mint: ctx.accounts.m_mint.to_account_info(),
                        current_authority: ctx.accounts.old_token_authority.to_account_info(),
                    },
                    &[&[b"token_authority", &[ctx.bumps.old_token_authority]]],
                ),
                AuthorityType::MintTokens,
                Some(ctx.accounts.new_token_authority.key()),
            )?;
        }

        Ok(())
    }
}
