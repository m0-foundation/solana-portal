use anchor_lang::prelude::*;
use common::{order_book, BridgeAdapter, BridgeError, FillReportPayload, Payload};

use crate::{
    instructions::send_message,
    state::{PortalGlobal, AUTHORITY_SEED, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct SendFillReport<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    /// Only order_book can send fill reports
    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        seeds::program = order_book::ID,
        bump,
    )]
    pub order_book_global: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        constraint = !portal_global.paused @ BridgeError::Paused,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    /// CHECK: account does not hold data
    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    pub portal_authority: UncheckedAccount<'info>,

    pub bridge_adapter: Interface<'info, BridgeAdapter>,

    pub system_program: Program<'info, System>,
}

impl SendFillReport<'_> {
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, SendFillReport<'info>>,
        order_id: [u8; 32],
        token_in: [u8; 32],
        amount_in_to_release: u128,
        amount_out_filled: u128,
        origin_recipient: [u8; 32],
        origin_chain_id: u32,
    ) -> Result<()> {
        let message_id = ctx.accounts.portal_global.generate_message_id();

        let message = Payload::FillReport(FillReportPayload {
            order_id,
            amount_in_to_release,
            amount_out_filled,
            origin_recipient,
            token_in,
            destination_chain_id: origin_chain_id,
            message_id,
        });

        send_message(
            ctx.accounts.bridge_adapter.to_account_info(),
            ctx.accounts.sender.to_account_info(),
            ctx.accounts.portal_authority.to_account_info(),
            ctx.bumps.portal_authority,
            ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts.to_vec(),
            message.encode(),
            origin_chain_id,
        )?;

        emit!(FillReportSent {
            destination_chain_id: origin_chain_id,
            bridge_adapter: ctx.accounts.bridge_adapter.key(),
            order_id,
            amount_in_to_release,
            amount_out_filled,
            origin_recipient,
            token_in,
            message_id,
        });

        Ok(())
    }
}

#[event]
pub struct FillReportSent {
    pub destination_chain_id: u32,
    pub bridge_adapter: Pubkey,
    pub order_id: [u8; 32],
    pub amount_in_to_release: u128,
    pub amount_out_filled: u128,
    pub origin_recipient: [u8; 32],
    pub token_in: [u8; 32],
    pub message_id: [u8; 32],
}
