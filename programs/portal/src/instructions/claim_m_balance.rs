use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface};
use common::BridgeError;

use crate::state::{PortalGlobal, AUTHORITY_SEED, GLOBAL_SEED};

#[derive(Accounts)]
pub struct ClaimMBalance<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

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

    /// Destination token account for claimed M tokens (admin specifies any valid M token account)
    #[account(
        mut,
        token::mint = m_mint,
        token::token_program = m_token_program,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    pub m_token_program: Interface<'info, TokenInterface>,
}

impl ClaimMBalance<'_> {
    pub fn handler(ctx: Context<Self>, amount: Option<u64>) -> Result<()> {
        let unclaimed = ctx.accounts.portal_global.unclaimed_m_balance;

        let claim_amount: u64 = match amount {
            Some(amt) => {
                require!((amt as u128) <= unclaimed, BridgeError::InvalidAmount);
                amt
            }
            None => unclaimed
                .try_into()
                .map_err(|_| BridgeError::InvalidAmount)?,
        };

        require!(claim_amount > 0, BridgeError::InvalidAmount);

        // Transfer M tokens from Portal authority ATA to destination
        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.m_token_program.to_account_info(),
                token_interface::TransferChecked {
                    from: ctx.accounts.authority_m_token_account.to_account_info(),
                    to: ctx.accounts.destination_token_account.to_account_info(),
                    authority: ctx.accounts.portal_authority.to_account_info(),
                    mint: ctx.accounts.m_mint.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.portal_authority]]],
            ),
            claim_amount,
            ctx.accounts.m_mint.decimals,
        )?;

        // Decrement the tracked balance
        ctx.accounts.portal_global.unclaimed_m_balance = ctx
            .accounts
            .portal_global
            .unclaimed_m_balance
            .checked_sub(claim_amount as u128)
            .ok_or(BridgeError::InvalidAmount)?;

        msg!(
            "Claimed {} M tokens. Remaining unclaimed balance: {}",
            claim_amount,
            ctx.accounts.portal_global.unclaimed_m_balance
        );

        emit!(MBalanceClaimed {
            admin: ctx.accounts.admin.key(),
            amount: claim_amount,
            destination: ctx.accounts.destination_token_account.key(),
            remaining_unclaimed: ctx.accounts.portal_global.unclaimed_m_balance,
        });

        Ok(())
    }
}

#[event]
pub struct MBalanceClaimed {
    pub admin: Pubkey,
    pub amount: u64,
    pub destination: Pubkey,
    pub remaining_unclaimed: u128,
}
