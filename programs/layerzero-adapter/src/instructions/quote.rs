use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::Instruction,
    program::{get_return_data, invoke},
};
use m0_portal_common::BridgeError;

use crate::{
    consts::QUOTE_DISCRIMINATOR,
    state::{LayerZeroGlobal, MessagingFee, QuoteParams, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct Quote<'info> {
    #[account(
        seeds = [GLOBAL_SEED],
        bump = lz_global.bump,
    )]
    pub lz_global: Account<'info, LayerZeroGlobal>,

    /// CHECK: Validated against lz_global.endpoint_program
    #[account(
        constraint = endpoint_program.key() == lz_global.endpoint_program @ BridgeError::InvalidBridgeAdapter,
    )]
    pub endpoint_program: AccountInfo<'info>,
}

impl<'info> Quote<'info> {
    pub fn handler(
        ctx: Context<'_, '_, '_, 'info, Self>,
        dst_eid: u32,
        receiver: [u8; 32],
        message: Vec<u8>,
        options: Vec<u8>,
        pay_in_lz_token: bool,
    ) -> Result<()> {
        let remaining = ctx.remaining_accounts;
        let endpoint_key = ctx.accounts.endpoint_program.key();

        let quote_params = QuoteParams {
            sender: ctx.accounts.lz_global.key(),
            dst_eid,
            receiver,
            message,
            options,
            pay_in_lz_token,
        };

        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&QUOTE_DISCRIMINATOR);
        instruction_data.extend_from_slice(&quote_params.try_to_vec()?);

        let mut accounts = Vec::with_capacity(remaining.len());
        for account in remaining.iter() {
            if account.is_writable {
                accounts.push(AccountMeta::new(account.key(), account.is_signer));
            } else {
                accounts.push(AccountMeta::new_readonly(account.key(), account.is_signer));
            }
        }

        let quote_ix = Instruction {
            program_id: endpoint_key,
            data: instruction_data,
            accounts,
        };

        let account_infos: Vec<AccountInfo> = remaining.to_vec();

        invoke(&quote_ix, &account_infos)?;

        // Read return data (MessagingFee)
        let (returning_program_id, returned_data) =
            get_return_data().ok_or(BridgeError::InvalidReturnData)?;

        require!(
            returning_program_id == endpoint_key,
            BridgeError::InvalidReturnData
        );

        let fee = MessagingFee::try_from_slice(&returned_data)
            .map_err(|_| error!(BridgeError::InvalidReturnData))?;

        // Set return data for the caller
        let bytes = fee.try_to_vec()?;
        anchor_lang::solana_program::program::set_return_data(&bytes);

        Ok(())
    }
}
