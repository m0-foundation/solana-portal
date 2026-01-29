use anchor_lang::prelude::*;
use anchor_lang::solana_program::program;
use m0_portal_common::pda;

use crate::{
    instructions::{SerializableAccountMeta, SimulationReturnData},
    state::{HyperlaneGlobal, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct GetIsm<'info> {
    #[account(
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,
}

impl GetIsm<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        // Uses default ISM if there is no return data or Option::None was returned
        // https://github.com/hyperlane-xyz/hyperlane-monorepo/blob/18b43e96d14f9023f25047cb9820079731eeff14/rust/sealevel/programs/mailbox/src/processor.rs#L488
        program::set_return_data(&ctx.accounts.hyperlane_global.ism.try_to_vec()?[..]);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct GetIsmMetas {}

impl GetIsmMetas {
    pub fn handler(_: Context<Self>) -> Result<()> {
        let account_metas: Vec<SerializableAccountMeta> =
            vec![AccountMeta::new_readonly(pda!(&[GLOBAL_SEED], &crate::ID), false).into()];

        let bytes = SimulationReturnData::new(account_metas)
            .try_to_vec()
            .map_err(|err| ProgramError::BorshIoError(err.to_string()))?;

        program::set_return_data(&bytes[..]);

        Ok(())
    }
}
