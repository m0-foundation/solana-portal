use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, get_associated_token_address_with_program_id},
    token, token_2022,
};

use crate::{
    ext_swap::{self, constants::GLOBAL_SEED},
    order_book::{self, constants::ORDER_SEED_PREFIX},
    pda, portal, BridgeError, Extension, Payload, AUTHORITY_SEED,
};

/// Returns account metas required to process the given payload.
/// Accounts will be passed as remaining accounts to the receive_message instruction on Portal.
pub fn require_metas(
    payload: &Payload,
    payer: Pubkey,
    extensions: Option<Vec<Extension>>,
    m_mint: Option<Pubkey>,
    orderbook_token_in: Option<&AccountInfo>,
) -> Result<Vec<AccountMeta>> {
    match payload {
        Payload::TokenTransfer(token_transfer) => {
            let extensions = extensions.ok_or(BridgeError::MissingOptionalAccount)?;
            let m_mint = m_mint.ok_or(BridgeError::MissingOptionalAccount)?;

            if extensions.is_empty() {
                msg!("No whitelisted extensions");
                return err!(BridgeError::InvalidSwapConfig);
            }

            // Find the extension program ID based on the destination mint
            let ext_program = extensions
                    .iter()
                    .find(|ext| ext.mint.eq(&token_transfer.destination_token.into()))
                    .unwrap_or_else(|| {
                        // If the extension program is not found, fallback to first whitelisted extension
                        let fallback = &extensions[0];
                        msg!(
                            "Extension for {} not found, falling back to first whitelisted extension: {}",
                            Pubkey::from(token_transfer.destination_token).to_string(),
                            fallback.mint.to_string(),
                        );
                        fallback
                    });

            let &Extension {
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
                &m_mint,
                &token_2022::ID,
            );
            let authority_m_token_account = get_associated_token_address_with_program_id(
                &pda!(&[AUTHORITY_SEED], &portal::ID),
                &m_mint,
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
            let token_in = Pubkey::from(report.token_in);

            let (token_in_program, unknown_token_program) =
                if let Some(order_token_in_info) = orderbook_token_in {
                    (*order_token_in_info.owner, false)
                } else {
                    // Default to SPL Token program if not provided
                    (token::ID, true)
                };

            // PDAs
            let order = pda!(&[ORDER_SEED_PREFIX, &report.order_id], &order_book::ID);
            let event_auth = pda!(&[b"__event_authority"], &order_book::ID);

            // Token accounts
            let recipient_token_account = get_associated_token_address_with_program_id(
                &report.origin_recipient.into(),
                &token_in,
                &token_in_program,
            );
            let order_token_account =
                get_associated_token_address_with_program_id(&order, &token_in, &token_in_program);

            let mut accounts = vec![
                AccountMeta::new(payer, false),
                AccountMeta::new(order, false),
                AccountMeta::new_readonly(token_in, false),
                AccountMeta::new_readonly(report.origin_recipient.into(), false),
                AccountMeta::new(recipient_token_account, false),
                AccountMeta::new(order_token_account, false),
                AccountMeta::new_readonly(token_in_program, false),
                AccountMeta::new_readonly(associated_token::ID, false),
                AccountMeta::new_readonly(order_book::ID, false),
                AccountMeta::new_readonly(event_auth, false),
            ];

            // Append token accounts in case token program guess was wrong
            if unknown_token_program {
                let recipient_token_account = get_associated_token_address_with_program_id(
                    &report.origin_recipient.into(),
                    &token_in,
                    &token_2022::ID,
                );
                let order_token_account = get_associated_token_address_with_program_id(
                    &order,
                    &token_in,
                    &token_2022::ID,
                );
                accounts.extend([
                    AccountMeta::new(recipient_token_account, false),
                    AccountMeta::new(order_token_account, false),
                    AccountMeta::new_readonly(token_2022::ID, false),
                ]);
            }

            Ok(accounts)
        }
        _ => Ok(vec![]),
    }
}
