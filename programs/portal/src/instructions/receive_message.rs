use anchor_lang::prelude::*;
use anchor_spl::token_interface;
use common::{
    earn::{self, cpi::accounts::PropagateIndex},
    ext_swap,
    order_book::{self, types::FillReport},
    BridgeAdapter, BridgeError, EarnerMerkleRootPayload, FillReportPayload, Payload,
    TokenTransferPayload,
};

use crate::state::{PortalGlobal, AUTHORITY_SEED, GLOBAL_SEED};

#[derive(Accounts)]
pub struct ReceiveMessage<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    pub adapter_authority: Signer<'info>,

    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    /// CHECK: account does not hold data
    pub portal_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl ReceiveMessage<'_> {
    fn validate(&self) -> Result<()> {
        // Check that one of the supported adapters signed the message
        if !BridgeAdapter::is_authority(&self.adapter_authority.key()) {
            return err!(BridgeError::InvalidAdapterAuthority);
        }

        Ok(())
    }

    #[access_control(ctx.accounts.validate())]
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        payload: Vec<u8>,
    ) -> Result<()> {
        let message = Payload::decode(&payload);

        match message {
            Payload::TokenTransfer(payload) => {
                msg!("Received Token Transfer Payload");
                ctx.accounts.portal_global.update_index(payload.index);
                return Self::handle_token_transfer_payload(ctx, payload);
            }
            Payload::Index(payload) => {
                msg!("Received Index Payload: {}", payload.index);
                ctx.accounts.portal_global.update_index(payload.index);
                return Self::handle_index_payload(&ctx, payload.into());
            }
            Payload::EarnerMerkleRoot(payload) => {
                msg!("Received EarnerMerkleRoot Payload");
                ctx.accounts.portal_global.update_index(payload.index);
                return Self::handle_index_payload(&ctx, payload);
            }
            Payload::FillReport(fill_report) => {
                msg!("Received Fill Report Payload");
                return Self::handle_fill_report_payload(ctx, fill_report);
            }
        }
    }

    fn handle_index_payload<'info>(
        ctx: &Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        payload: EarnerMerkleRootPayload,
    ) -> Result<()> {
        let accounts = payload.parse_and_validate_accounts(ctx.remaining_accounts.to_vec())?;
        let authority_seed: &[&[&[u8]]] = &[&[AUTHORITY_SEED, &[ctx.bumps.portal_authority]]];

        let propogate_ctx = CpiContext::new_with_signer(
            accounts.earn_program.to_account_info(),
            PropagateIndex {
                signer: ctx.accounts.portal_authority.to_account_info(),
                global_account: accounts.m_global,
                m_mint: accounts.m_mint,
                token_program: accounts.m_token_program,
            },
            authority_seed,
        );

        msg!("Index update: {}", payload.index);
        earn::cpi::propagate_index(propogate_ctx, payload.index, payload.merkle_root)
    }

    fn handle_token_transfer_payload<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        payload: TokenTransferPayload,
    ) -> Result<()> {
        let accounts = payload.parse_and_validate_accounts(ctx.remaining_accounts.to_vec())?;
        let amount = payload.amount;

        if payload.index > 0 {
            Self::handle_index_payload(&ctx, payload.into())?;
        }

        // Get the principal amount of $M tokens to transfer using the multiplier
        let principal = common::amount_to_principal_down(
            amount,
            common::get_scaled_ui_config(&accounts.m_mint)?
                .new_multiplier
                .into(),
        );

        // Mint to authority account which will wrap it to the recipient
        token_interface::mint_to(
            CpiContext::new_with_signer(
                accounts.m_token_program.to_account_info(),
                token_interface::MintTo {
                    mint: accounts.m_mint.to_account_info(),
                    to: accounts.authority_m_token_account.clone(),
                    authority: ctx.accounts.portal_authority.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.portal_authority]]],
            ),
            principal.try_into().unwrap(),
        )?;

        // Wrap $M to extension tokens
        ext_swap::cpi::wrap(
            CpiContext::new_with_signer(
                accounts.swap_program,
                ext_swap::cpi::accounts::Wrap {
                    signer: ctx.accounts.portal_authority.to_account_info(),
                    wrap_authority: Some(ctx.accounts.portal_authority.to_account_info()),
                    swap_global: accounts.swap_global,
                    to_global: accounts.extension_global,
                    to_mint: accounts.extension_mint,
                    m_mint: accounts.m_mint,
                    m_token_account: accounts.authority_m_token_account,
                    to_token_account: accounts.recipient_token_account,
                    to_m_vault_auth: accounts.extension_m_vault_authority,
                    to_mint_authority: accounts.extension_mint_authority,
                    to_m_vault: accounts.extension_m_vault,
                    to_token_program: accounts.extension_token_program,
                    m_token_program: accounts.m_token_program,
                    to_ext_program: accounts.extension_program,
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.portal_authority]]],
            ),
            principal.try_into().unwrap(),
        )?;

        Ok(())
    }

    fn handle_fill_report_payload<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        payload: FillReportPayload,
    ) -> Result<()> {
        // Get payload specific accounts
        let accounts = payload.parse_and_validate_accounts(ctx.remaining_accounts.to_vec())?;

        order_book::cpi::report_order_fill(
            CpiContext::new_with_signer(
                accounts.orderbook_program.clone(),
                order_book::cpi::accounts::ReportOrderFill {
                    relayer: ctx.accounts.sender.to_account_info(),
                    messenger_authority: ctx.accounts.portal_authority.to_account_info(),
                    global_account: accounts.orderbook_global_account,
                    order: accounts.order,
                    token_in_mint: accounts.token_in_mint,
                    origin_recipient: accounts.origin_recipient,
                    recipient_token_in_ata: accounts.recipient_token_in_ata,
                    order_token_in_ata: accounts.order_token_in_ata,
                    token_in_program: accounts.token_in_program,
                    associated_token_program: accounts.associated_token_program,
                    system_program: ctx.accounts.system_program.to_account_info(),
                    event_authority: accounts.event_authority,
                    program: accounts.orderbook_program,
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.portal_authority]]],
            ),
            FillReport {
                order_id: payload.order_id,
                amount_in_to_release: payload.amount_in_to_release,
                amount_out_filled: payload.amount_out_filled,
                origin_recipient: payload.origin_recipient,
            },
        )
    }
}
