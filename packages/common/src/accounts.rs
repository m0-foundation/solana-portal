use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use crate::{FillReportPayload, TokenTransferPayload};

const SWAP_PROGRAM: Pubkey = pubkey!("MSwapi3WhNKMUGm9YrxGhypgUEt7wYQH3ZgG32XoWzH");
const ORDERBOOK_PROGRAM: Pubkey = pubkey!("4Qgxc6VkBGaAQAikirnkApYNyy1W6asQgMHZxKgRcSL8");

// Helper macro to extract accounts from remaining_accounts vector
macro_rules! extract_accounts {
    ($struct_type:ident, $accounts:expr, { $($field:ident : $idx:expr),* $(,)? }) => {{
        $struct_type {
            $(
                $field: $accounts
                    .get($idx)
                    .cloned()
                    .ok_or_else(|| {
                        msg!("Missing account at index: {}", $idx);
                        CommonError::MissingOptionalAccount
                    })?,
            )*
        }
    }};
}

#[error_code]
pub enum CommonError {
    #[msg("Missing optional account required for payload type")]
    MissingOptionalAccount,
    #[msg("Remaining account invalid")]
    InvalidRemainingAccount,
}

pub struct TokenTransferPayloadAccounts<'info> {
    pub extension_mint: AccountInfo<'info>,
    pub recipient_token_account: AccountInfo<'info>,
    pub authority_m_token_account: AccountInfo<'info>,
    pub extension_m_vault: AccountInfo<'info>,
    pub extension_m_vault_authority: AccountInfo<'info>,
    pub extension_mint_authority: AccountInfo<'info>,
    pub extension_global: AccountInfo<'info>,
    pub extension_token_program: AccountInfo<'info>,
    pub extension_program: AccountInfo<'info>,
    pub swap_global: AccountInfo<'info>,
    pub swap_program: AccountInfo<'info>,
}

impl TokenTransferPayload {
    pub fn parse_and_validate_accounts<'info>(
        &self,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<TokenTransferPayloadAccounts<'info>> {
        let accounts = extract_accounts!(TokenTransferPayloadAccounts, &remaining_accounts, {
            extension_mint: 0,
            recipient_token_account: 1,
            authority_m_token_account: 2,
            extension_m_vault: 3,
            extension_m_vault_authority: 4,
            extension_mint_authority: 5,
            extension_global: 6,
            extension_token_program: 7,
            extension_program: 8,
            swap_global: 9,
            swap_program: 10,
        });

        // Recipient matches transfer payload
        let recipient_token_account = get_associated_token_address_with_program_id(
            &Pubkey::from(self.recipient),
            accounts.extension_mint.key,
            accounts.extension_token_program.key,
        );
        if accounts.recipient_token_account.key() != recipient_token_account {
            return err!(CommonError::InvalidRemainingAccount);
        }

        if accounts.swap_program.key != &SWAP_PROGRAM {
            return err!(CommonError::InvalidRemainingAccount);
        }

        Ok(accounts)
    }
}

pub struct FillReportPayloadAccounts<'info> {
    pub orderbook_global_account: AccountInfo<'info>,
    pub order: AccountInfo<'info>,
    pub token_in_mint: AccountInfo<'info>,
    pub origin_recipient: AccountInfo<'info>,
    pub recipient_token_in_ata: AccountInfo<'info>,
    pub order_token_in_ata: AccountInfo<'info>,
    pub token_in_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
    pub event_authority: AccountInfo<'info>,
    pub orderbook_program: AccountInfo<'info>,
}

impl FillReportPayload {
    pub fn parse_and_validate_accounts<'info>(
        &self,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<FillReportPayloadAccounts<'info>> {
        let accounts = extract_accounts!(FillReportPayloadAccounts, &remaining_accounts, {
          orderbook_global_account: 0,
          order: 1,
          token_in_mint: 2,
          origin_recipient: 3,
          recipient_token_in_ata: 4,
          order_token_in_ata: 5,
          token_in_program: 6,
          associated_token_program: 7,
          event_authority: 8,
          orderbook_program: 9,
        });

        if accounts.orderbook_program.key != &ORDERBOOK_PROGRAM {
            return err!(CommonError::InvalidRemainingAccount);
        }

        Ok(accounts)
    }
}
