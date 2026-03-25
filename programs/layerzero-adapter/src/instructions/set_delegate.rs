use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use m0_portal_common::BridgeError;

use crate::{
    consts::SET_DELEGATE_DISCRIMINATOR,
    state::{LayerZeroGlobal, SetDelegateParams, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct SetDelegate<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = lz_global.bump,
        has_one = admin,
    )]
    pub lz_global: Account<'info, LayerZeroGlobal>,

    /// CHECK: Validated against lz_global.endpoint_program
    #[account(
        constraint = endpoint_program.key() == lz_global.endpoint_program @ BridgeError::InvalidBridgeAdapter,
    )]
    pub endpoint_program: AccountInfo<'info>,
}

impl<'info> SetDelegate<'info> {
    pub fn handler(
        ctx: Context<'_, '_, '_, 'info, Self>,
        delegate: Pubkey,
    ) -> Result<()> {
        let remaining = ctx.remaining_accounts;
        let endpoint_key = ctx.accounts.endpoint_program.key();

        let params = SetDelegateParams { delegate };

        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&SET_DELEGATE_DISCRIMINATOR);
        instruction_data.extend_from_slice(&params.try_to_vec()?);

        // Set delegate accounts:
        // 0: oapp/lz_global (signer via seeds)
        // 1: oapp_registry (writable)
        // 2: event_authority
        // 3: endpoint_program
        let lz_global_key = ctx.accounts.lz_global.key();

        let mut accounts = vec![
            AccountMeta::new_readonly(lz_global_key, true),
        ];

        for account in remaining.iter() {
            if account.is_writable {
                accounts.push(AccountMeta::new(account.key(), account.is_signer));
            } else {
                accounts.push(AccountMeta::new_readonly(account.key(), account.is_signer));
            }
        }

        let set_delegate_ix = Instruction {
            program_id: endpoint_key,
            data: instruction_data,
            accounts,
        };

        let mut account_infos = Vec::with_capacity(remaining.len() + 1);
        account_infos.push(ctx.accounts.lz_global.to_account_info());
        for account in remaining.iter() {
            account_infos.push(account.to_account_info());
        }

        let bump = ctx.accounts.lz_global.bump;
        invoke_signed(
            &set_delegate_ix,
            &account_infos,
            &[&[GLOBAL_SEED, &[bump]]],
        )?;

        Ok(())
    }
}
