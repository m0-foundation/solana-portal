use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface};
use m0_portal_common::{
    ext_swap::{self, accounts::SwapGlobal, program::ExtSwap},
    BridgeAdapter, BridgeError, PayloadData, TokenTransferPayload,
};

use crate::{
    instructions::send_message,
    state::{PortalGlobal, AUTHORITY_SEED, GLOBAL_SEED, MINT_AUTHORITY_SEED, M_VAULT_SEED},
};

#[derive(Accounts)]
pub struct SendToken<'info> {
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
        seeds = [GLOBAL_SEED],
        seeds::program = ext_swap::ID,
        bump = swap_global.bump,
    )]
    pub swap_global: Account<'info, SwapGlobal>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        seeds::program = extension_program.key(),
        bump,
    )]
    /// CHECK: account checked on uwrap CPI
    pub extension_global: AccountInfo<'info>,

    #[account(
        mut,
        mint::token_program = m_token_program
    )]
    pub m_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub extension_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = m_mint,
        associated_token::authority = portal_authority,
        associated_token::token_program = m_token_program,
    )]
    pub m_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub extension_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    /// CHECK: account does not hold data
    pub portal_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = m_mint,
        associated_token::authority = ext_m_vault_auth,
        associated_token::token_program = m_token_program,
    )]
    pub ext_m_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [M_VAULT_SEED],
        seeds::program = extension_program.key(),
        bump,
    )]
    /// CHECK: account does not hold data
    pub ext_m_vault_auth: AccountInfo<'info>,

    #[account(
        seeds = [MINT_AUTHORITY_SEED],
        seeds::program = extension_program.key(),
        bump,
    )]
    /// CHECK: account does not hold data
    pub ext_mint_authority: AccountInfo<'info>,

    pub swap_program: Program<'info, ExtSwap>,

    /// CHECK: account checked on uwrap CPI
    pub extension_program: AccountInfo<'info>,

    pub m_token_program: Interface<'info, TokenInterface>,

    pub extension_token_program: Interface<'info, TokenInterface>,

    pub bridge_adapter: Interface<'info, BridgeAdapter>,

    pub system_program: Program<'info, System>,
}

impl SendToken<'_> {
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

        if self
            .swap_global
            .whitelisted_extensions
            .iter()
            .find(|ext| {
                ext.program_id == self.extension_program.key()
                    && ext.mint == self.extension_mint.key()
            })
            .is_none()
        {
            return err!(BridgeError::InvalidExtension);
        }

        if amount == 0 {
            return err!(BridgeError::InvalidAmount);
        }

        Ok(())
    }

    #[access_control(ctx.accounts.validate(amount, destination_chain_id))]
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, SendToken<'info>>,
        amount: u64,
        destination_token: [u8; 32],
        destination_chain_id: u32,
        recipient: [u8; 32],
    ) -> Result<()> {
        let m_pre_balance = ctx.accounts.m_token_account.amount;

        // Unwrap extension tokens to $M
        ext_swap::cpi::unwrap(
            CpiContext::new_with_signer(
                ctx.accounts.swap_program.to_account_info(),
                ext_swap::cpi::accounts::Unwrap {
                    signer: ctx.accounts.sender.to_account_info(),
                    unwrap_authority: Some(ctx.accounts.portal_authority.to_account_info()),
                    swap_global: ctx.accounts.swap_global.to_account_info(),
                    from_global: ctx.accounts.extension_global.to_account_info(),
                    from_mint: ctx.accounts.extension_mint.to_account_info(),
                    m_mint: ctx.accounts.m_mint.to_account_info(),
                    m_token_account: ctx.accounts.m_token_account.to_account_info(),
                    from_token_account: ctx.accounts.extension_token_account.to_account_info(),
                    from_m_vault_auth: ctx.accounts.ext_m_vault_auth.to_account_info(),
                    from_mint_authority: ctx.accounts.ext_mint_authority.to_account_info(),
                    from_m_vault: ctx.accounts.ext_m_vault.to_account_info(),
                    from_token_program: ctx.accounts.extension_token_program.to_account_info(),
                    m_token_program: ctx.accounts.m_token_program.to_account_info(),
                    from_ext_program: ctx.accounts.extension_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.portal_authority]]],
            ),
            amount,
        )?;

        // Amount of $M we got from unwrap
        ctx.accounts.m_token_account.reload()?;
        let m_amount = ctx.accounts.m_token_account.amount - m_pre_balance;

        // Burn $M
        token_interface::burn(
            CpiContext::new_with_signer(
                ctx.accounts.m_token_program.to_account_info(),
                token_interface::Burn {
                    mint: ctx.accounts.m_mint.to_account_info(),
                    from: ctx.accounts.m_token_account.to_account_info(),
                    authority: ctx.accounts.portal_authority.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.portal_authority]]],
            ),
            m_amount,
        )?;

        let scaled_m_amount = m0_portal_common::principal_to_amount_down(
            m_amount,
            m0_portal_common::get_scaled_ui_config(&ctx.accounts.m_mint.to_account_info())?
                .multiplier
                .into(),
        );

        let payload = PayloadData::TokenTransfer(TokenTransferPayload {
            amount: scaled_m_amount,
            destination_token,
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
            source_token: ctx.accounts.extension_mint.key(),
            destination_chain_id,
            destination_token,
            sender: ctx.accounts.sender.key(),
            recipient,
            amount: m_amount as u128,
            index: ctx.accounts.portal_global.m_index,
            bridge_adapter: ctx.accounts.bridge_adapter.key(),
        });

        Ok(())
    }
}

#[event]
pub struct TokenSent {
    pub source_token: Pubkey,
    pub destination_chain_id: u32,
    pub destination_token: [u8; 32],
    pub sender: Pubkey,
    pub recipient: [u8; 32],
    pub amount: u128,
    pub index: u128,
    pub bridge_adapter: Pubkey,
}
