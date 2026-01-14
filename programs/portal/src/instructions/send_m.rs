use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface};
use common::{BridgeAdapter, BridgeError, PayloadData, TokenTransferPayload};

use crate::{
    instructions::{send_message, TokenSent},
    state::{PortalGlobal, AUTHORITY_SEED, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct SendM<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        constraint = !portal_global.outgoing_paused @ BridgeError::Paused,
        has_one = m_mint @ BridgeError::InvalidMint,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        mut,
        mint::token_program = m_token_program
    )]
    pub m_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = m_mint,
        associated_token::authority = portal_authority,
        associated_token::token_program = m_token_program,
    )]
    pub m_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    /// CHECK: account does not hold data
    pub portal_authority: UncheckedAccount<'info>,

    pub m_token_program: Interface<'info, TokenInterface>,

    pub bridge_adapter: Interface<'info, BridgeAdapter>,

    pub system_program: Program<'info, System>,
}

impl SendM<'_> {
    fn validate(&self, amount: u64, destination_chain_id: u32) -> Result<()> {
        if self.portal_global.outgoing_paused {
            return err!(BridgeError::Paused);
        }

        // Only allow sending to hub if spoke is isolated
        if let Some(chain_id) = self.portal_global.isolated_hub_chain_id {
            if chain_id != destination_chain_id {
                return err!(BridgeError::InvalidTransfer);
            }
        }

        if amount == 0 {
            return err!(BridgeError::InvalidAmount);
        }

        Ok(())
    }

    #[access_control(ctx.accounts.validate(amount, destination_chain_id))]
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, SendM<'info>>,
        amount: u64,
        destination_chain_id: u32,
        recipient: [u8; 32],
    ) -> Result<()> {
        // Burn $M
        token_interface::burn(
            CpiContext::new(
                ctx.accounts.m_token_program.to_account_info(),
                token_interface::Burn {
                    mint: ctx.accounts.m_mint.to_account_info(),
                    from: ctx.accounts.m_token_account.to_account_info(),
                    authority: ctx.accounts.sender.to_account_info(),
                },
            ),
            amount,
        )?;

        let scaled_m_amount = common::principal_to_amount_down(
            amount as u128,
            common::get_scaled_ui_config(&ctx.accounts.m_mint.to_account_info())?
                .multiplier
                .into(),
        );

        let payload = PayloadData::TokenTransfer(TokenTransferPayload {
            amount: scaled_m_amount,
            destination_token: ctx.accounts.portal_global.evm_m_mint,
            sender: ctx.accounts.sender.key().to_bytes(),
            recipient,
            index: ctx.accounts.portal_global.m_index,
        });

        // Send message to bridge adapter
        send_message(
            ctx.accounts.bridge_adapter.to_account_info(),
            ctx.accounts.sender.to_account_info(),
            ctx.accounts.portal_authority.to_account_info(),
            ctx.bumps.portal_authority,
            ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts.to_vec(),
            destination_chain_id,
            ctx.accounts
                .portal_global
                .generate_message_id(destination_chain_id),
            payload,
            PayloadData::TOKEN_TRANSFER_DISCRIMINANT,
        )?;

        emit!(TokenSent {
            source_token: ctx.accounts.portal_global.m_mint,
            destination_chain_id,
            destination_token: ctx.accounts.portal_global.evm_m_mint,
            sender: ctx.accounts.sender.key(),
            recipient,
            amount: scaled_m_amount,
            index: ctx.accounts.portal_global.m_index,
            bridge_adapter: ctx.accounts.bridge_adapter.key(),
        });

        Ok(())
    }
}
