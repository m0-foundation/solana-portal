use anchor_lang::prelude::*;

use crate::{
    instructions::create_lut,
    state::{WormholeGlobal, GLOBAL_SEED},
};

#[derive(Accounts)]
#[instruction(recent_slot: u64)]
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

    /// CHECK: lut account validated by lut program
    #[account(
        mut,
        seeds = [wormhole_global.key().as_ref(), &recent_slot.to_le_bytes()],
        seeds::program = lut_program,
        bump
    )]
    pub lut_address: UncheckedAccount<'info>,

    /// CHECK: lut program
    #[account(
        executable,
        address = solana_address_lookup_table_interface::program::ID
    )]
    pub lut_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl Initialize<'_> {
    pub fn handler(ctx: Context<Self>, recent_slot: u64) -> Result<()> {
        create_lut(
            recent_slot,
            vec![],
            ctx.accounts.lut_address.to_account_info(),
            ctx.accounts.admin.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.wormhole_global.to_account_info(),
            ctx.bumps.wormhole_global,
        )?;

        ctx.accounts.wormhole_global.set_inner(WormholeGlobal {
            bump: ctx.bumps.wormhole_global,
            admin: ctx.accounts.admin.key(),
            paused: false,
            peers: Vec::new(),
            pending_admin: None,
            receive_lut: ctx.accounts.lut_address.key(),
        });

        Ok(())
    }
}
