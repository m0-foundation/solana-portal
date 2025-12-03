use anchor_lang::prelude::*;
use common::pda;

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
        program::set_return_data(&ctx.accounts.hyperlane_global.ism.try_to_vec()?[..]);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct GetIsmMetas<'info> {
    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,
}

impl GetIsmMetas<'_> {
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
