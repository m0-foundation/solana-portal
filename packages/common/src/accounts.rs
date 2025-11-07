use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use common_macros::ExtractAccounts;

use crate::{ext_swap, order_book, CommonError, FillReportPayload, TokenTransferPayload};

#[derive(ExtractAccounts)]
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
        let accounts =
            TokenTransferPayloadAccounts::extract_from_remaining_accounts(&remaining_accounts)?;

        // Recipient matches transfer payload
        let recipient_token_account = get_associated_token_address_with_program_id(
            &Pubkey::from(self.recipient),
            accounts.extension_mint.key,
            accounts.extension_token_program.key,
        );
        if accounts.recipient_token_account.key() != recipient_token_account {
            return err!(CommonError::InvalidRemainingAccount);
        }

        if accounts.swap_program.key != &ext_swap::ID {
            return err!(CommonError::InvalidRemainingAccount);
        }

        Ok(accounts)
    }
}

#[derive(ExtractAccounts)]
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
        let accounts =
            FillReportPayloadAccounts::extract_from_remaining_accounts(&remaining_accounts)?;

        if accounts.orderbook_program.key != &order_book::ID {
            return err!(CommonError::InvalidRemainingAccount);
        }

        Ok(accounts)
    }
}
