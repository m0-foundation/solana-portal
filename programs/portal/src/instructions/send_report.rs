use anchor_lang::prelude::*;
use m0_portal_common::{
    order_book, BridgeAdapter, BridgeError, CancelReportPayload, FillReportPayload, PayloadData,
};

use crate::{
    instructions::send_message,
    state::{PortalGlobal, GLOBAL_SEED, PORTAL_AUTHORITY_SEED},
};

#[derive(Accounts)]
pub struct SendReport<'info> {
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
        constraint = !portal_global.outgoing_paused @ BridgeError::Paused,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    /// CHECK: account does not hold data
    #[account(
        seeds = [PORTAL_AUTHORITY_SEED],
        bump,
    )]
    pub portal_authority: UncheckedAccount<'info>,

    pub bridge_adapter: Interface<'info, BridgeAdapter>,

    pub system_program: Program<'info, System>,
}

impl SendReport<'_> {
    pub fn send_fill_report_handler<'info>(
        ctx: Context<'_, '_, '_, 'info, SendReport<'info>>,
        order_id: [u8; 32],
        token_in: [u8; 32],
        amount_in_to_release: u128,
        amount_out_filled: u128,
        origin_recipient: [u8; 32],
        origin_chain_id: u32,
    ) -> Result<()> {
        let payload = PayloadData::FillReport(FillReportPayload {
            order_id,
            amount_in_to_release,
            amount_out_filled,
            origin_recipient,
            token_in,
        });

        send_message(
            ctx.accounts.bridge_adapter.to_account_info(),
            ctx.accounts.sender.to_account_info(),
            &mut ctx.accounts.portal_global,
            ctx.accounts.portal_authority.to_account_info(),
            ctx.bumps.portal_authority,
            ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts.to_vec(),
            origin_chain_id,
            payload,
            PayloadData::FILL_REPORT_DISCRIMINANT,
        )?;

        emit!(FillReportSent {
            destination_chain_id: origin_chain_id,
            bridge_adapter: ctx.accounts.bridge_adapter.key(),
            order_id,
            amount_in_to_release,
            amount_out_filled,
            origin_recipient,
            token_in,
        });

        Ok(())
    }

    pub fn send_cancel_report_handler<'info>(
        ctx: Context<'_, '_, '_, 'info, SendReport<'info>>,
        order_id: [u8; 32],
        order_sender: [u8; 32],
        token_in: [u8; 32],
        amount_in_to_refund: u128,
        origin_chain_id: u32,
    ) -> Result<()> {
        let payload = PayloadData::CancelReport(CancelReportPayload {
            order_id,
            order_sender,
            token_in,
            amount_in_to_refund,
        });

        send_message(
            ctx.accounts.bridge_adapter.to_account_info(),
            ctx.accounts.sender.to_account_info(),
            &mut ctx.accounts.portal_global,
            ctx.accounts.portal_authority.to_account_info(),
            ctx.bumps.portal_authority,
            ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts.to_vec(),
            origin_chain_id,
            payload,
            PayloadData::CANCEL_REPORT_DISCRIMINANT,
        )?;

        emit!(CancelReportSent {
            destination_chain_id: origin_chain_id,
            bridge_adapter: ctx.accounts.bridge_adapter.key(),
            order_id,
            order_sender,
            token_in,
            amount_in_to_refund,
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
}

#[event]
pub struct CancelReportSent {
    pub destination_chain_id: u32,
    pub bridge_adapter: Pubkey,
    pub order_id: [u8; 32],
    pub order_sender: [u8; 32],
    pub token_in: [u8; 32],
    pub amount_in_to_refund: u128,
}
