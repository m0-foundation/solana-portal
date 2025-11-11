use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, get_associated_token_address_with_program_id},
    token_2022,
};
use common::{
    ext_swap::{self, types::WhitelistedExtension},
    order_book::{self, accounts::NativeOrder},
    pda,
    portal::{self, program::Portal},
    require_metas, BridgeError, Payload, AUTHORITY_SEED,
};

use crate::{
    consts::MAILBOX_PROGRAM_ID,
    instructions::{SerializableAccountMeta, SimulationReturnData},
    state::{HyperlaneGlobal, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct ReceiveMessage<'info> {
    #[account(
        mut,
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
        mut,
        seeds = [AUTHORITY_SEED],
        bump
    )]
    /// CHECK: Account does not hold data
    /// Payer for Portal CPI (needs to be funded)
    pub hyperlane_adapter_authority: AccountInfo<'info>,

    #[account(
        constraint = !hyperlane_global.paused,
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

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
                    sender: ctx.accounts.hyperlane_adapter_authority.to_account_info(),
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
        ctx: Context<Self>,
        _origin: u32,
        _sender: [u8; 32],
        message: Vec<u8>,
    ) -> Result<()> {
        let payload = Payload::decode(&message);

        let hyperlane_adapter_authority = pda!(&[AUTHORITY_SEED], &crate::ID);
        let hyperlane_global = pda!(&[GLOBAL_SEED], &crate::ID);
        let messenger_authority = pda!(&[AUTHORITY_SEED], &portal::ID);

        // Accounts needed by all payload types
        let mut account_metas: Vec<SerializableAccountMeta> = vec![
            AccountMeta::new(hyperlane_adapter_authority, false).into(),
            AccountMeta::new_readonly(hyperlane_global, false).into(),
            AccountMeta::new_readonly(messenger_authority, false).into(),
            AccountMeta::new_readonly(portal::ID, false).into(),
            AccountMeta::new_readonly(system_program::ID, false).into(),
        ];

        let required_remaining = require_metas(
            &payload,
            RESOLVER_PUBKEY_PAYER,
            swap_global_data,
            earn_global_data,
            order_data,
            order_token_in,
        )?;

        // Add expected remaining accounts based on payload type
        account_metas.extend(required_remaining.iter().cloned().map(|a| a.into()));

        let bytes = SimulationReturnData::new(account_metas)
            .try_to_vec()
            .map_err(|err| ProgramError::BorshIoError(err.to_string()))?;

        program::set_return_data(&bytes[..]);

        Ok(())
    }
}
