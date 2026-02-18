use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use m0_portal_common::{
    ext_swap::{self, accounts::SwapGlobal, program::ExtSwap},
    m_ext::constants::{MINT_AUTHORITY_SEED, M_VAULT_SEED},
    BridgeError,
};

use crate::state::{PortalGlobal, AUTHORITY_SEED, GLOBAL_SEED};

#[derive(Accounts)]
pub struct WrapUnclaimed<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
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
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    /// CHECK: PDA authority, does not hold data
    pub portal_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = m_mint,
        associated_token::authority = portal_authority,
        associated_token::token_program = m_token_program,
    )]
    pub authority_m_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(address = portal_global.m_mint)]
    pub m_mint: InterfaceAccount<'info, Mint>,

    pub extension_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = extension_mint,
        token::token_program = extension_token_program,
    )]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = m_mint,
        associated_token::authority = extension_m_vault_authority,
        associated_token::token_program = m_token_program,
    )]
    pub extension_m_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [MINT_AUTHORITY_SEED],
        seeds::program = extension_program.key(),
        bump,
    )]
    /// CHECK: account does not hold data
    pub extension_mint_authority: AccountInfo<'info>,

    #[account(
        seeds = [M_VAULT_SEED],
        seeds::program = extension_program.key(),
        bump,
    )]
    /// CHECK: account does not hold data
    pub extension_m_vault_authority: AccountInfo<'info>,

    pub m_token_program: Interface<'info, TokenInterface>,

    pub extension_token_program: Interface<'info, TokenInterface>,

    /// CHECK: account checked on uwrap CPI
    pub extension_program: AccountInfo<'info>,

    pub swap_program: Program<'info, ExtSwap>,

    pub system_program: Program<'info, System>,
}

impl WrapUnclaimed<'_> {
    pub fn handler(ctx: Context<Self>, amount: Option<u64>) -> Result<()> {
        let unclaimed = ctx.accounts.portal_global.unclaimed_m_balance;
        let claim_amount = amount.unwrap_or(unclaimed);

        require!(claim_amount > 0, BridgeError::InvalidAmount);
        require!(unclaimed >= claim_amount, BridgeError::InvalidAmount);

        // Wrap $M tokens to an extension designated by the admin
        ext_swap::cpi::wrap(
            CpiContext::new_with_signer(
                ctx.accounts.swap_program.to_account_info(),
                ext_swap::cpi::accounts::Wrap {
                    signer: ctx.accounts.portal_authority.to_account_info(),
                    wrap_authority: Some(ctx.accounts.portal_authority.to_account_info()),
                    swap_global: ctx.accounts.swap_global.to_account_info(),
                    to_global: ctx.accounts.extension_global.to_account_info(),
                    to_mint: ctx.accounts.extension_mint.to_account_info(),
                    m_mint: ctx.accounts.m_mint.to_account_info(),
                    m_token_account: ctx.accounts.authority_m_token_account.to_account_info(),
                    to_token_account: ctx.accounts.recipient_token_account.to_account_info(),
                    to_m_vault_auth: ctx.accounts.extension_m_vault_authority.to_account_info(),
                    to_mint_authority: ctx.accounts.extension_mint_authority.to_account_info(),
                    to_m_vault: ctx.accounts.extension_m_vault.to_account_info(),
                    to_token_program: ctx.accounts.extension_token_program.to_account_info(),
                    m_token_program: ctx.accounts.m_token_program.to_account_info(),
                    to_ext_program: ctx.accounts.extension_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.portal_authority]]],
            ),
            claim_amount,
        )?;

        // Decrement the tracked balance
        ctx.accounts.portal_global.unclaimed_m_balance -= claim_amount;

        emit!(MBalanceClaimed {
            admin: ctx.accounts.admin.key(),
            amount: claim_amount,
            destination: ctx.accounts.recipient_token_account.key(),
            remaining_unclaimed: unclaimed - claim_amount,
        });

        Ok(())
    }
}

#[event]
pub struct MBalanceClaimed {
    pub admin: Pubkey,
    pub amount: u64,
    pub destination: Pubkey,
    pub remaining_unclaimed: u64,
}
