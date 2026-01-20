use anchor_lang::prelude::{
    program::{invoke, invoke_signed},
    *,
};
use m0_portal_common::{earn, ext_swap, pda, portal, wormhole_verify_vaa_shim};
use solana_address_lookup_table_interface::instruction::{
    create_lookup_table, extend_lookup_table,
};

use crate::{
    consts::AUTHORITY_SEED,
    state::{WormholeGlobal, GLOBAL_SEED},
};

#[derive(Accounts)]
#[instruction(recent_slot: u64)]
pub struct SetLookupTable<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = wormhole_global.bump,
        has_one = admin,
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

impl SetLookupTable<'_> {
    pub fn handler(ctx: Context<Self>, recent_slot: u64) -> Result<()> {
        let (ix, _) = create_lookup_table(
            ctx.accounts.wormhole_global.key(),
            ctx.accounts.admin.key(),
            recent_slot,
        );

        invoke(
            &ix,
            &[
                ctx.accounts.lut_address.to_account_info(),
                ctx.accounts.wormhole_global.to_account_info(),
                ctx.accounts.admin.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        let ix = extend_lookup_table(
            ctx.accounts.lut_address.key(),
            ctx.accounts.wormhole_global.key(),
            Some(ctx.accounts.admin.key()),
            vec![
                pda!(&[GLOBAL_SEED], &crate::ID),
                pda!(&[GLOBAL_SEED], &portal::ID),
                pda!(&[GLOBAL_SEED], &earn::ID),
                pda!(&[GLOBAL_SEED], &ext_swap::ID),
                pda!(&[AUTHORITY_SEED], &crate::ID),
                pda!(&[AUTHORITY_SEED], &portal::ID),
                portal::ID,
                wormhole_verify_vaa_shim::ID,
                system_program::ID,
            ],
        );

        invoke_signed(
            &ix,
            &[
                ctx.accounts.lut_address.to_account_info(),
                ctx.accounts.wormhole_global.to_account_info(),
                ctx.accounts.admin.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[&[GLOBAL_SEED, &[ctx.accounts.wormhole_global.bump]]],
        )?;

        ctx.accounts.wormhole_global.receive_lut = Some(ctx.accounts.lut_address.key());

        Ok(())
    }
}
