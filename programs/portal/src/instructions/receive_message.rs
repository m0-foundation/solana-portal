use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface};
use common::{Payload, TokenTransferPayload};

use crate::{
    errors::PortalError,
    instructions::{
        earn::{self, accounts::EarnGlobal, cpi::accounts::PropagateIndex, program::Earn},
        ext_swap::{self, accounts::SwapGlobal, program::ExtSwap},
    },
    required_optional,
    state::{AUTHORITY_SEED, GLOBAL_SEED},
    unwrap_or_default,
};

#[derive(Accounts)]
pub struct ReceiveMessage<'info> {
    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    /// CHECK: account does not hold data
    pub messenger_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        seeds::program = earn::ID,
        bump = m_global.bump,
        has_one = m_mint,
    )]
    pub m_global: Account<'info, EarnGlobal>,

    #[account(mut)]
    pub m_mint: InterfaceAccount<'info, Mint>,

    pub earn_program: Program<'info, Earn>,

    pub m_token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,

    /*
     * Optional accounts for TokenTransfer payload
     */
    #[account(mut)]
    pub extension_mint: Option<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = extension_mint,
        associated_token::authority = messenger_authority,
        associated_token::token_program = extension_token_program,
    )]
    pub recipient_token_account: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = m_mint,
        associated_token::authority = messenger_authority,
        associated_token::token_program = m_token_program,
    )]
    /// Transient $M account that is allowed to hold tokens before wrapping
    pub authority_m_token_account: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = m_mint,
        associated_token::authority = extension_m_vault_authority,
        associated_token::token_program = m_token_program,
    )]
    pub extension_m_vault: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        seeds = [b"m_vault"],
        seeds::program = unwrap_or_default!(extension_program),
        bump,
    )]
    /// CHECK: account does not hold data
    pub extension_m_vault_authority: Option<AccountInfo<'info>>,

    #[account(
        seeds = [b"mint_authority"],
        seeds::program = unwrap_or_default!(extension_program),
        bump,
    )]
    /// CHECK: account does not hold data
    pub extension_mint_authority: Option<AccountInfo<'info>>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        seeds::program = unwrap_or_default!(extension_program),
        bump,
    )]
    /// CHECK: wrap CPI will validate the account
    pub extension_global: Option<AccountInfo<'info>>,

    pub extension_token_program: Option<Interface<'info, TokenInterface>>,

    /// CHECK: checked against whitelisted extensions
    pub extension_program: Option<AccountInfo<'info>>,

    #[account(
        seeds = [GLOBAL_SEED],
        seeds::program = ext_swap::ID,
        bump = swap_global.bump,
    )]
    pub swap_global: Option<Account<'info, SwapGlobal>>,

    pub swap_program: Option<Program<'info, ExtSwap>>,
}

impl ReceiveMessage<'_> {
    pub fn handler(ctx: Context<Self>, payload: Vec<u8>) -> Result<()> {
        let message = Payload::decode(payload);

        match message {
            Payload::TokenTransfer(token_transfer) => {
                msg!("Received Token Transfer Payload");
                return Self::handle_token_transfer_payload(ctx, token_transfer);
            }
            Payload::Index(index_payload) => {
                msg!("Received Index Payload: {}", index_payload.index);
                return Self::handle_index_payload(&ctx, index_payload.index);
            }
            Payload::FillReport(_fill_report) => {
                msg!("Received Fill Report Payload");
            }
        }

        Ok(())
    }

    fn handle_index_payload(ctx: &Context<Self>, index: u64) -> Result<()> {
        let authority_seed: &[&[&[u8]]] = &[&[AUTHORITY_SEED, &[ctx.bumps.messenger_authority]]];

        let propogate_ctx = CpiContext::new_with_signer(
            ctx.accounts.earn_program.to_account_info(),
            PropagateIndex {
                signer: ctx.accounts.messenger_authority.to_account_info(),
                global_account: ctx.accounts.m_global.to_account_info(),
                m_mint: ctx.accounts.m_mint.to_account_info(),
                token_program: ctx.accounts.m_token_program.to_account_info(),
            },
            authority_seed,
        );

        earn::cpi::propagate_index(propogate_ctx, index, [0; 32])?;
        msg!("Index update: {}", index);

        Ok(())
    }

    fn handle_token_transfer_payload(
        ctx: Context<Self>,
        payload: TokenTransferPayload,
    ) -> Result<()> {
        if payload.index > 0 {
            Self::handle_index_payload(&ctx, payload.index)?;

            // Reload the mint to ensure the latest multiplier is used
            ctx.accounts.m_mint.reload()?;
        }

        // Unwrap optional accounts that are required for token transfer
        let recipient_token_account = required_optional!(ctx.accounts.recipient_token_account);
        let authority_m_token_account = required_optional!(ctx.accounts.authority_m_token_account);
        let extension_mint = required_optional!(ctx.accounts.extension_mint);
        let swap_global = required_optional!(ctx.accounts.swap_global);
        let swap_program = required_optional!(ctx.accounts.swap_program);
        let extension_token_program = required_optional!(ctx.accounts.extension_token_program);
        let extension_program = required_optional!(ctx.accounts.extension_program);
        let extension_global = required_optional!(ctx.accounts.extension_global);
        let extension_m_vault_auth = required_optional!(ctx.accounts.extension_m_vault_authority);
        let extension_mint_authority = required_optional!(ctx.accounts.extension_mint_authority);
        let extension_m_vault = required_optional!(ctx.accounts.extension_m_vault);

        // Ensure target extensions is correct
        // (only validate if we know the extension exists)
        let target_mint = Pubkey::from(payload.destination_token);
        if swap_global
            .whitelisted_extensions
            .iter()
            .find(|ext| ext.mint.eq(&target_mint))
            .is_some()
        {
            if !extension_mint.key().eq(&target_mint) {
                return err!(PortalError::InvalidMint);
            }
        }

        let scaled_ui_config = common::get_scaled_ui_config(&ctx.accounts.m_mint)?;

        // Get the principal amount of $M tokens to transfer using the multiplier
        let principal = common::amount_to_principal_down(
            payload.amount,
            scaled_ui_config.new_multiplier.into(),
        );

        // Mint to authority account which will wrap it to the recipient
        token_interface::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.m_token_program.to_account_info(),
                token_interface::MintTo {
                    mint: ctx.accounts.m_mint.to_account_info(),
                    to: authority_m_token_account.to_account_info(),
                    authority: ctx.accounts.messenger_authority.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.messenger_authority]]],
            ),
            principal.try_into().unwrap(),
        )?;

        // Wrap $M to extension tokens
        ext_swap::cpi::wrap(
            CpiContext::new_with_signer(
                swap_program.to_account_info(),
                ext_swap::cpi::accounts::Wrap {
                    signer: ctx.accounts.messenger_authority.to_account_info(),
                    wrap_authority: Some(ctx.accounts.messenger_authority.to_account_info()),
                    swap_global: swap_global.to_account_info(),
                    to_global: extension_global.to_account_info(),
                    to_mint: extension_mint.to_account_info(),
                    m_mint: ctx.accounts.m_mint.to_account_info(),
                    m_token_account: authority_m_token_account.to_account_info(),
                    to_token_account: recipient_token_account.to_account_info(),
                    to_m_vault_auth: extension_m_vault_auth.to_account_info(),
                    to_mint_authority: extension_mint_authority.to_account_info(),
                    to_m_vault: extension_m_vault.to_account_info(),
                    to_token_program: extension_token_program.to_account_info(),
                    m_token_program: ctx.accounts.m_token_program.to_account_info(),
                    to_ext_program: extension_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.messenger_authority]]],
            ),
            principal.try_into().unwrap(),
        )?;

        Ok(())
    }
}
