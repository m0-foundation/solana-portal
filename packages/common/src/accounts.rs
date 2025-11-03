use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use crate::TokenTransferPayload;

macro_rules! extract_accounts {
    ($accounts:expr, { $($field:ident : $idx:expr),* $(,)? }) => {{
        TokenTransferPayloadAccounts {
            $(
                $field: $accounts.get($idx).cloned().ok_or_else(|| CommonError::MissingOptionalAccount)?,
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
        let accounts = extract_accounts!(&remaining_accounts, {
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

        Ok(accounts)
    }
}
