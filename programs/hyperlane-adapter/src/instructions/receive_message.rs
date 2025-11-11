use anchor_lang::prelude::*;
use common::{
    pda,
    portal::{self, program::Portal},
    BridgeError, AUTHORITY_SEED,
};

use crate::{
    consts::MAILBOX_PROGRAM_ID,
    instructions::{SerializableAccountMeta, SimulationReturnData},
    state::{HyperlaneGlobal, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct ReceiveMessage<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(
        constraint = !hyperlane_global.paused,
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

    #[account(
        seeds = [AUTHORITY_SEED],
        bump
    )]
    /// CHECK: Account does not hold data
    pub hyperlane_adapter_authority: AccountInfo<'info>,

    #[account(
        seeds = [
            b"hyperlane",
            b"-",
            b"process_authority",
            b"-",
            crate::ID.as_ref(),
        ],
        seeds::program = MAILBOX_PROGRAM_ID,
        bump
    )]
    pub hyperlane_process_authority: Signer<'info>,

    #[account(
        seeds = [AUTHORITY_SEED],
        seeds::program = portal::ID,
        bump
    )]
    /// CHECK: Account does not hold data
    pub messenger_authority: AccountInfo<'info>,

    pub portal_program: Program<'info, Portal>,

    pub system_program: Program<'info, System>,
}

impl ReceiveMessage<'_> {
    fn validate(&self, origin: u32, sender: &[u8; 32]) -> Result<()> {
        let peer = self.hyperlane_global.get_peer_by_chain_id(origin)?;

        if &peer.address != sender {
            return err!(BridgeError::InvalidPeer);
        }

        Ok(())
    }

    #[access_control(ctx.accounts.validate(origin, &sender))]
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        origin: u32,
        sender: [u8; 32],
        message: Vec<u8>,
    ) -> Result<()> {
        portal::cpi::receive_message(
            CpiContext::new(
                ctx.accounts.portal_program.to_account_info(),
                portal::cpi::accounts::ReceiveMessage {
                    sender: ctx.accounts.relayer.to_account_info(),
                    adapter_authority: ctx.accounts.hyperlane_adapter_authority.to_account_info(),
                    messenger_authority: ctx.accounts.messenger_authority.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            )
            .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
            message,
        )
    }
}

#[derive(Accounts)]
pub struct ReceiveMessageMetas {}

impl ReceiveMessageMetas {
    pub fn handler<'info>(
        _ctx: Context<Self>,
        _origin: u32,
        _sender: [u8; 32],
        _message: Vec<u8>,
    ) -> Result<()> {
        let hyperlane_adapter_authority = pda!(&[AUTHORITY_SEED], &crate::ID);

        let account_metas: Vec<SerializableAccountMeta> =
            vec![AccountMeta::new(hyperlane_adapter_authority, false).into()];

        let bytes = SimulationReturnData::new(account_metas)
            .try_to_vec()
            .map_err(|err| ProgramError::BorshIoError(err.to_string()))?;

        program::set_return_data(&bytes[..]);

        Ok(())
    }
}
