use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use m0_portal_common::Peers;

use crate::{
    consts::REGISTER_OAPP_DISCRIMINATOR,
    state::{LayerZeroGlobal, RegisterOAppParams, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = LayerZeroGlobal::size(0),
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub lz_global: Account<'info, LayerZeroGlobal>,

    /// CHECK: Validated as executable; stored in global state
    #[account(executable)]
    pub endpoint_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn handler(ctx: Context<'_, '_, '_, 'info, Self>, chain_id: u32) -> Result<()> {
        let bump = ctx.bumps.lz_global;

        ctx.accounts.lz_global.set_inner(LayerZeroGlobal {
            bump,
            admin: ctx.accounts.admin.key(),
            pending_admin: None,
            chain_id,
            endpoint_program: ctx.accounts.endpoint_program.key(),
            outgoing_paused: false,
            incoming_paused: false,
            peers: Peers::default(),
            padding: [0u8; 128],
        });

        // Register as OApp with the LZ endpoint.
        // remaining_accounts: [oapp_registry, event_authority]
        let remaining = ctx.remaining_accounts;

        let params = RegisterOAppParams {
            delegate: ctx.accounts.admin.key(),
        };

        let mut instruction_data = Vec::with_capacity(8 + 32);
        instruction_data.extend_from_slice(&REGISTER_OAPP_DISCRIMINATOR);
        instruction_data.extend_from_slice(&params.try_to_vec()?);

        let endpoint_key = ctx.accounts.endpoint_program.key();

        let register_ix = Instruction {
            program_id: endpoint_key,
            data: instruction_data,
            accounts: vec![
                AccountMeta::new(ctx.accounts.admin.key(), true),
                AccountMeta::new_readonly(ctx.accounts.lz_global.key(), true),
                AccountMeta::new(remaining[0].key(), false), // oapp_registry
                AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
                AccountMeta::new_readonly(remaining[1].key(), false), // event_authority
                AccountMeta::new_readonly(endpoint_key, false),
            ],
        };

        let account_infos = vec![
            ctx.accounts.admin.to_account_info(),
            ctx.accounts.lz_global.to_account_info(),
            remaining[0].to_account_info(), // oapp_registry
            ctx.accounts.system_program.to_account_info(),
            remaining[1].to_account_info(), // event_authority
            ctx.accounts.endpoint_program.to_account_info(),
        ];

        invoke_signed(
            &register_ix,
            &account_infos,
            &[&[GLOBAL_SEED, &[bump]]],
        )?;

        Ok(())
    }
}
