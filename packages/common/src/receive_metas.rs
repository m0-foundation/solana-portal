use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, get_associated_token_address_with_program_id},
    token, token_2022,
};

use crate::{
    earn,
    ext_swap::{self, constants::GLOBAL_SEED},
    order_book::{self, constants::ORDER_SEED_PREFIX},
    pda,
    portal::{
        self,
        constants::{MINT_AUTHORITY_SEED, M_VAULT_SEED},
    },
    wormhole_adapter::constants::EVENT_AUTHORITY_SEED,
    BridgeError, Extension, PayloadData, AUTHORITY_SEED,
};

/// Returns account metas required to process the given payload.
/// Accounts will be passed as remaining accounts to the receive_message instruction on Portal.
pub fn require_metas(
    payload: &PayloadData,
    extensions: Option<Vec<Extension>>,
    m_mint: Option<Pubkey>,
    orderbook_token_in: Option<&AccountInfo>,
) -> Result<Vec<AccountMeta>> {
    match payload {
        PayloadData::TokenTransfer(token_transfer) => {
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
            let extension_m_vault_auth = pda!(&[M_VAULT_SEED], &extension_pid);
            let extension_mint_auth = pda!(&[MINT_AUTHORITY_SEED], &extension_pid);
            let extension_global = pda!(&[GLOBAL_SEED], &extension_pid);
            let swap_global = pda!(&[GLOBAL_SEED], &ext_swap::ID);
            let m_global = pda!(&[GLOBAL_SEED], &earn::ID);

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
                AccountMeta::new(m_global, false),
                AccountMeta::new(m_mint, false),
                AccountMeta::new_readonly(earn::ID, false),
                AccountMeta::new_readonly(token_2022::ID, false),
                AccountMeta::new(extension_mint, false),
                AccountMeta::new(recipient_token_account, false),
                AccountMeta::new(authority_m_token_account, false),
                AccountMeta::new(extention_m_vault, false),
                AccountMeta::new_readonly(extension_m_vault_auth, false),
                AccountMeta::new_readonly(extension_mint_auth, false),
                AccountMeta::new(extension_global, false),
                AccountMeta::new_readonly(extension_token_program, false),
                AccountMeta::new_readonly(extension_pid, false),
                AccountMeta::new_readonly(swap_global, false),
                AccountMeta::new_readonly(ext_swap::ID, false),
            ])
        }
        PayloadData::Index(_) | PayloadData::EarnerMerkleRoot(_) => {
            let m_global = pda!(&[GLOBAL_SEED], &earn::ID);
            let m_mint = m_mint.ok_or(BridgeError::MissingOptionalAccount)?;

            Ok(vec![
                AccountMeta::new(m_global, false),
                AccountMeta::new(m_mint, false),
                AccountMeta::new_readonly(earn::ID, false),
                AccountMeta::new_readonly(token_2022::ID, false),
            ])
        }
        PayloadData::FillReport(_) | PayloadData::CancelReport(_) => {
            // Extract common fields and determine recipient based on report type
            let (token_in, order_id, recipient) = match payload {
                PayloadData::FillReport(r) => (r.token_in, r.order_id, r.origin_recipient),
                PayloadData::CancelReport(r) => (r.token_in, r.order_id, r.order_sender),
                _ => unreachable!(),
            };

            let token_in = Pubkey::from(token_in);

            let token_in_program = orderbook_token_in
                .map(|account| *account.owner)
                .or_else(|| {
                    // Check if token is an extension and get its token program
                    extensions.and_then(|exts| {
                        exts.iter()
                            .find(|ext| ext.mint == token_in)
                            .map(|ext| ext.token_program)
                    })
                })
                // Default to SPL Token program if no other info is available
                .unwrap_or(token::ID);

            // PDAs
            let order = pda!(&[ORDER_SEED_PREFIX, &order_id], &order_book::ID);
            let event_auth = pda!(&[EVENT_AUTHORITY_SEED], &order_book::ID);
            let orderbook_global = pda!(&[GLOBAL_SEED], &order_book::ID);

            // Token accounts
            let recipient_token_account = get_associated_token_address_with_program_id(
                &recipient.into(),
                &token_in,
                &token_in_program,
            );
            let order_token_account =
                get_associated_token_address_with_program_id(&order, &token_in, &token_in_program);

            let mut accounts = vec![
                AccountMeta::new_readonly(orderbook_global, false),
                AccountMeta::new(order, false),
                AccountMeta::new_readonly(token_in, false),
                AccountMeta::new_readonly(recipient.into(), false),
                AccountMeta::new(recipient_token_account, false),
                AccountMeta::new(order_token_account, false),
                AccountMeta::new_readonly(token_in_program, false),
                AccountMeta::new_readonly(associated_token::ID, false),
                AccountMeta::new_readonly(event_auth, false),
                AccountMeta::new_readonly(order_book::ID, false),
            ];

            // Append token accounts in case token program guess was wrong
            if orderbook_token_in.is_none() {
                let recipient_token_account = get_associated_token_address_with_program_id(
                    &recipient.into(),
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
    }
}
