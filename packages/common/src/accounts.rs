use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use borsh::{BorshDeserialize, BorshSerialize};
use common_macros::ExtractAccounts;

use crate::{
    earn, ext_swap, order_book, BridgeError, FillReportPayload, IndexPayload, RegistrarListPayload,
    TokenTransferPayload,
};

#[derive(ExtractAccounts)]
pub struct IndexPayloadAccounts<'info> {
    pub m_global: AccountInfo<'info>,
    pub m_mint: AccountInfo<'info>,
    pub earn_program: AccountInfo<'info>,
    pub m_token_program: AccountInfo<'info>,
}

impl IndexPayload {
    pub fn parse_and_validate_accounts<'info>(
        &self,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<IndexPayloadAccounts<'info>> {
        let accounts = IndexPayloadAccounts::extract_from_remaining_accounts(&remaining_accounts)?;

        if accounts.earn_program.key != &earn::ID {
            return err!(BridgeError::InvalidRemainingAccount);
        }

        Ok(accounts)
    }
}

#[derive(ExtractAccounts)]
pub struct RegistrarListPayloadAccounts<'info> {
    pub earn_program: AccountInfo<'info>,
    pub m_global: AccountInfo<'info>,
    pub user: AccountInfo<'info>,
    pub m_mint: AccountInfo<'info>,
    pub user_token_account: AccountInfo<'info>,
    pub m_token_program: AccountInfo<'info>,
    pub associated_token_program: AccountInfo<'info>,
}

impl RegistrarListPayload {
    pub fn parse_and_validate_accounts<'info>(
        &self,
        remaining_accounts: Vec<AccountInfo<'info>>,
    ) -> Result<RegistrarListPayloadAccounts<'info>> {
        let accounts =
            RegistrarListPayloadAccounts::extract_from_remaining_accounts(&remaining_accounts)?;

        // Ensure the registrar address matches the payload
        if accounts.user.key() != Pubkey::from(self.address) {
            return err!(BridgeError::InvalidRemainingAccount);
        }

        if accounts.earn_program.key != &earn::ID {
            return err!(BridgeError::InvalidRemainingAccount);
        }

        if accounts.associated_token_program.key != &anchor_spl::associated_token::ID {
            return err!(BridgeError::InvalidRemainingAccount);
        }

        Ok(accounts)
    }
}

#[derive(ExtractAccounts)]
pub struct TokenTransferPayloadAccounts<'info> {
    // Shared with IndexPayloadAccounts
    pub m_global: AccountInfo<'info>,
    pub m_mint: AccountInfo<'info>,
    pub earn_program: AccountInfo<'info>,
    pub m_token_program: AccountInfo<'info>,
    // Remaining accounts specific to TokenTransferPayload
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
            return err!(BridgeError::InvalidRemainingAccount);
        }

        if accounts.swap_program.key != &ext_swap::ID {
            return err!(BridgeError::InvalidRemainingAccount);
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
            return err!(BridgeError::InvalidRemainingAccount);
        }

        Ok(accounts)
    }
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct Extension {
    pub program_id: Pubkey,
    pub mint: Pubkey,
    pub token_program: Pubkey,
}

impl Extension {
    pub const SIZE: usize = 96;
}

impl From<ext_swap::types::WhitelistedExtension> for Extension {
    fn from(ext: ext_swap::types::WhitelistedExtension) -> Self {
        Extension {
            program_id: Pubkey::from(ext.program_id),
            mint: Pubkey::from(ext.mint),
            token_program: Pubkey::from(ext.token_program),
        }
    }
}

#[cfg(feature = "idl-build")]
use anchor_lang_idl::types::{
    Idl, IdlDefinedFields, IdlField, IdlSerialization, IdlType, IdlTypeDef, IdlTypeDefTy,
};

#[cfg(feature = "idl-build")]
impl anchor_lang::IdlBuild for Extension {
    fn create_type() -> Option<IdlTypeDef> {
        Some(IdlTypeDef {
            name: "Extension".to_string(),
            docs: vec![],
            serialization: IdlSerialization::Borsh,
            repr: None,
            generics: vec![],
            ty: IdlTypeDefTy::Struct {
                fields: Some(IdlDefinedFields::Named(vec![
                    IdlField {
                        name: "program_id".to_string(),
                        docs: Default::default(),
                        ty: IdlType::Pubkey,
                    },
                    IdlField {
                        name: "mint".to_string(),
                        docs: Default::default(),
                        ty: IdlType::Pubkey,
                    },
                    IdlField {
                        name: "token_program".to_string(),
                        docs: Default::default(),
                        ty: IdlType::Pubkey,
                    },
                ])),
            },
        })
    }
}
