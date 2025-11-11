use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, get_associated_token_address_with_program_id},
    token_2022,
};

use crate::{
    earn::accounts::EarnGlobal,
    ext_swap::{self, accounts::SwapGlobal, constants::GLOBAL_SEED, types::WhitelistedExtension},
    order_book::{self, accounts::NativeOrder, constants::ORDER_SEED_PREFIX},
    pda, portal, BridgeError, Payload, AUTHORITY_SEED,
};

/// Returns account metas required to process the given payload.
/// Accounts will be passed as remaining accounts to the receive_message instruction on Portal.
pub fn require_metas(
    payload: &Payload,
    payer: Pubkey,
    token_transfer_swap_global_data: Option<SwapGlobal>,
    token_transfer_earn_global_data: Option<EarnGlobal>,
    orderbook_order_data: Option<NativeOrder>,
    orderbook_token_in: Option<AccountInfo>,
) -> Result<Vec<AccountMeta>> {
    match payload {
        Payload::TokenTransfer(token_transfer) => {
            let swap_global_data =
                token_transfer_swap_global_data.ok_or(BridgeError::MissingOptionalAccount)?;
            let earn_global_data =
                token_transfer_earn_global_data.ok_or(BridgeError::MissingOptionalAccount)?;

            if swap_global_data.whitelisted_extensions.is_empty() {
                msg!("No whitelisted extensions");
                return err!(BridgeError::InvalidSwapConfig);
            }

            // Find the extension program ID based on the destination mint
            let ext_program = swap_global_data
                    .whitelisted_extensions
                    .iter()
                    .find(|ext| ext.mint.eq(&token_transfer.destination_token.into()))
                    .unwrap_or_else(|| {
                        // If the extension program is not found, fallback to first whitelisted extension
                        let fallback = &swap_global_data.whitelisted_extensions[0];
                        msg!(
                            "Extension for {} not found, falling back to first whitelisted extension: {}",
                            Pubkey::from(token_transfer.destination_token).to_string(),
                            fallback.mint.to_string(),
                        );
                        fallback
                    });

            let &WhitelistedExtension {
                mint: extension_mint,
                program_id: extension_pid,
                token_program: extension_token_program,
            } = ext_program;

            // PDAs
            let extension_m_vault_auth = pda!(&[b"m_vault"], &extension_pid);
            let extension_mint_auth = pda!(&[b"mint_authority"], &extension_pid);
            let extension_global = pda!(&[GLOBAL_SEED], &extension_pid);
            let swap_global = pda!(&[GLOBAL_SEED], &ext_swap::ID);

            // Token accounts
            let recipient_token_account = get_associated_token_address_with_program_id(
                &token_transfer.recipient.into(),
                &extension_mint,
                &extension_token_program,
            );
            let extention_m_vault = get_associated_token_address_with_program_id(
                &extension_m_vault_auth,
                &earn_global_data.m_mint,
                &token_2022::ID,
            );
            let authority_m_token_account = get_associated_token_address_with_program_id(
                &pda!(&[AUTHORITY_SEED], &portal::ID),
                &earn_global_data.m_mint,
                &token_2022::ID,
            );

            Ok(vec![
                AccountMeta::new(extension_mint, false),
                AccountMeta::new(recipient_token_account, false),
                AccountMeta::new_readonly(authority_m_token_account, false),
                AccountMeta::new(extention_m_vault, false),
                AccountMeta::new_readonly(extension_m_vault_auth, false),
                AccountMeta::new_readonly(extension_mint_auth, false),
                AccountMeta::new_readonly(extension_global, false),
                AccountMeta::new_readonly(extension_token_program, false),
                AccountMeta::new_readonly(extension_pid, false),
                AccountMeta::new_readonly(swap_global, false),
                AccountMeta::new_readonly(ext_swap::ID, false),
            ])
        }
        Payload::FillReport(report) => {
            let order_data = orderbook_order_data.ok_or(BridgeError::MissingOptionalAccount)?;
            let token_in = orderbook_token_in.ok_or(BridgeError::MissingOptionalAccount)?;

            // PDAs
            let order = pda!(&[ORDER_SEED_PREFIX, &report.order_id], &order_book::ID);
            let event_auth = pda!(&[b"__event_authority"], &order_book::ID);

            let token_in_program = token_in.owner;

            // Token accounts
            let recipient_token_account = get_associated_token_address_with_program_id(
                &report.origin_recipient.into(),
                &order_data.token_in,
                token_in_program,
            );
            let order_token_account = get_associated_token_address_with_program_id(
                &order,
                &order_data.token_in,
                token_in_program,
            );

            Ok(vec![
                AccountMeta::new(payer, false),
                AccountMeta::new(order, false),
                AccountMeta::new_readonly(order_data.token_in, false),
                AccountMeta::new_readonly(report.origin_recipient.into(), false),
                AccountMeta::new(recipient_token_account, false),
                AccountMeta::new(order_token_account, false),
                AccountMeta::new_readonly(*token_in_program, false),
                AccountMeta::new_readonly(associated_token::ID, false),
                AccountMeta::new_readonly(order_book::ID, false),
                AccountMeta::new_readonly(event_auth, false),
            ])
        }
        _ => Ok(vec![]),
    }
}
