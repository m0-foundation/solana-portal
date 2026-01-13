use anchor_lang::prelude::*;
use anchor_spl::token_interface;
use common::{
    earn::{self, cpi::accounts::PropagateIndex},
    ext_swap,
    order_book::{
        self,
        types::{CancelReport, FillReport},
    },
    BridgeAdapter, BridgeError, CancelReportPayload, EarnerMerkleRootPayload, FillReportPayload,
    Payload, PayloadData, TokenTransferPayload,
};

use crate::state::{
    BridgeMessage, PortalGlobal, AUTHORITY_SEED, ETHEREUM_CHAIN_ID, GLOBAL_SEED, MESSAGE_SEED,
};

#[derive(Accounts)]
#[instruction(message_id: [u8; 32])]
pub struct ReceiveMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        constraint = !portal_global.incoming_paused @ BridgeError::Paused,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    pub adapter_authority: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = BridgeMessage::SIZE,
        seeds = [MESSAGE_SEED, &message_id],
        bump,
    )]
    pub message_account: Account<'info, BridgeMessage>,

    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    /// CHECK: account does not hold data
    pub portal_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl ReceiveMessage<'_> {
    fn validate(
        &self,
        message_id: [u8; 32],
        source_chain_id: u32,
        payload: &Vec<u8>,
    ) -> Result<()> {
        let message = Payload::decode(&payload)?;

        // Verify the message_id matches the decoded payload
        require!(
            message_id == message.header.message_id,
            BridgeError::InvalidMessageId
        );

        // Check that one of the supported adapters signed the message
        if BridgeAdapter::from_authority(&self.adapter_authority.key()).is_none() {
            return err!(BridgeError::InvalidAdapterAuthority);
        }

        // Make sure the message is intended for this chain
        if message.header.destination_chain_id != self.portal_global.chain_id {
            return err!(BridgeError::InvalidDestinationChain);
        }
        if !BridgeAdapter::valid_destination_peer(message.header.destination_peer) {
            return err!(BridgeError::InvalidDestinationPeer);
        }

        // Only accept Earner Merkle Root payloads from the mainnet hub
        if let PayloadData::EarnerMerkleRoot(_payload) = &message.data {
            if source_chain_id != ETHEREUM_CHAIN_ID {
                return err!(BridgeError::InvalidSourceChain);
            }
        }

        Ok(())
    }

    #[access_control(ctx.accounts.validate(message_id, source_chain_id, &payload))]
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        message_id: [u8; 32],
        source_chain_id: u32,
        payload: Vec<u8>,
    ) -> Result<()> {
        let message = Payload::decode(&payload)?;

        // Protect against replays in case the adapter is not
        ctx.accounts
            .message_account
            .set_inner(BridgeMessage { consumed: true });

        match message.data {
            PayloadData::TokenTransfer(payload) => {
                ctx.accounts
                    .portal_global
                    .update_index(message_id, payload.index);

                return Self::handle_token_transfer_payload(
                    ctx,
                    source_chain_id,
                    payload,
                    message.header.message_id,
                );
            }
            PayloadData::Index(payload) => {
                ctx.accounts
                    .portal_global
                    .update_index(message_id, payload.index);

                return Self::handle_index_payload(&ctx, payload.into());
            }
            PayloadData::EarnerMerkleRoot(payload) => {
                ctx.accounts
                    .portal_global
                    .update_index(message_id, payload.index);

                return Self::handle_index_payload(&ctx, payload);
            }

            PayloadData::FillReport(fill_report) => {
                return Self::handle_fill_report_payload(
                    ctx,
                    source_chain_id,
                    fill_report,
                    message.header.message_id,
                );
            }
            PayloadData::CancelReport(cancel_report) => {
                return Self::handle_cancel_report_payload(
                    ctx,
                    source_chain_id,
                    cancel_report,
                    message.header.message_id,
                );
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

        msg!(
            "Index update: {}, Merkle update: {}",
            payload.index,
            !payload.merkle_root.is_empty()
        );

        earn::cpi::propagate_index(
            propogate_ctx,
            payload
                .index
                .try_into()
                .expect("could not cast index to u64"),
            payload.merkle_root,
        )
    }

    fn handle_token_transfer_payload<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        source_chain_id: u32,
        payload: TokenTransferPayload,
        message_id: [u8; 32],
    ) -> Result<()> {
        // Tokens can only come from the hub if the spoke is isolated
        if let Some(chain_id) = ctx.accounts.portal_global.isolated_hub_chain_id {
            if chain_id != source_chain_id {
                return err!(BridgeError::InvalidTransfer);
            }
        }

        if payload.index > 0 {
            Self::handle_index_payload(&ctx, payload.clone().into())?;
        }

        let accounts = payload.parse_and_validate_accounts(
            ctx.remaining_accounts.to_vec(),
            ctx.accounts.portal_global.m_mint,
        )?;

        // Get the principal amount of $M tokens to transfer using the multiplier
        let principal = common::amount_to_principal_down(
            payload.amount,
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
                    signer: ctx.accounts.portal_authority.to_account_info(), // Signer owns the $M tokens
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

        emit!(TokenReceived {
            source_chain_id,
            bridge_adapter: *ctx.accounts.adapter_authority.as_ref().owner,
            destination_token: payload.destination_token,
            sender: payload.sender,
            recipient: payload.recipient,
            amount: payload.amount,
            index: payload.index,
            message_id: message_id,
        });

        Ok(())
    }

    fn handle_fill_report_payload<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        source_chain_id: u32,
        payload: FillReportPayload,
        message_id: [u8; 32],
    ) -> Result<()> {
        // Get payload specific accounts
        let accounts = payload.parse_and_validate_accounts(ctx.remaining_accounts.to_vec())?;

        order_book::cpi::report_order_fill(
            CpiContext::new_with_signer(
                accounts.orderbook_program.clone(),
                order_book::cpi::accounts::ReportOrderFill {
                    relayer: ctx.accounts.payer.to_account_info(), // Relayer pays for ATA initialization
                    portal_authority: ctx.accounts.portal_authority.to_account_info(),
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
            source_chain_id,
            FillReport {
                order_id: payload.order_id,
                amount_in_to_release: payload.amount_in_to_release,
                amount_out_filled: payload.amount_out_filled,
                origin_recipient: payload.origin_recipient,
                token_in: payload.token_in,
            },
        )?;

        emit!(FillReportReceived {
            source_chain_id: source_chain_id,
            bridge_adapter: *ctx.accounts.adapter_authority.as_ref().owner,
            order_id: payload.order_id,
            amount_in_to_release: payload.amount_in_to_release,
            amount_out_filled: payload.amount_out_filled,
            origin_recipient: payload.origin_recipient,
            token_in: payload.token_in,
            message_id: message_id,
        });

        Ok(())
    }

    fn handle_cancel_report_payload<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        source_chain_id: u32,
        payload: CancelReportPayload,
        message_id: [u8; 32],
    ) -> Result<()> {
        // Get payload specific accounts
        let accounts = payload.parse_and_validate_accounts(ctx.remaining_accounts.to_vec())?;

        order_book::cpi::report_order_cancel(
            CpiContext::new_with_signer(
                accounts.orderbook_program.clone(),
                order_book::cpi::accounts::ReportOrderCancel {
                    relayer: ctx.accounts.payer.to_account_info(), // Relayer pays for ATA initialization
                    portal_authority: ctx.accounts.portal_authority.to_account_info(),
                    global_account: accounts.orderbook_global_account,
                    order: accounts.order,
                    token_in_mint: accounts.token_in_mint,
                    order_sender: accounts.order_sender,
                    sender_token_in_ata: accounts.sender_token_in_ata,
                    order_token_in_ata: accounts.order_token_in_ata,
                    token_in_program: accounts.token_in_program,
                    associated_token_program: accounts.associated_token_program,
                    system_program: ctx.accounts.system_program.to_account_info(),
                    event_authority: accounts.event_authority,
                    program: accounts.orderbook_program,
                },
                &[&[AUTHORITY_SEED, &[ctx.bumps.portal_authority]]],
            ),
            source_chain_id,
            CancelReport {
                order_id: payload.order_id,
                order_sender: payload.order_sender,
                token_in: payload.token_in,
                amount_in_to_refund: payload.amount_in_to_refund,
            },
        )?;

        emit!(CancelReportReceived {
            source_chain_id: source_chain_id,
            bridge_adapter: *ctx.accounts.adapter_authority.as_ref().owner,
            order_id: payload.order_id,
            order_sender: payload.order_sender,
            token_in: payload.token_in,
            amount_in_to_refund: payload.amount_in_to_refund,
            message_id: message_id,
        });

        Ok(())
    }
}

#[event]
pub struct TokenReceived {
    pub source_chain_id: u32,
    pub bridge_adapter: Pubkey,
    pub destination_token: [u8; 32],
    pub sender: [u8; 32],
    pub recipient: [u8; 32],
    pub amount: u128,
    pub index: u128,
    pub message_id: [u8; 32],
}

#[event]
pub struct FillReportReceived {
    pub source_chain_id: u32,
    pub bridge_adapter: Pubkey,
    pub order_id: [u8; 32],
    pub amount_in_to_release: u128,
    pub amount_out_filled: u128,
    pub origin_recipient: [u8; 32],
    pub token_in: [u8; 32],
    pub message_id: [u8; 32],
}

#[event]
pub struct CancelReportReceived {
    pub source_chain_id: u32,
    pub bridge_adapter: Pubkey,
    pub order_id: [u8; 32],
    pub order_sender: [u8; 32],
    pub token_in: [u8; 32],
    pub amount_in_to_refund: u128,
    pub message_id: [u8; 32],
}
