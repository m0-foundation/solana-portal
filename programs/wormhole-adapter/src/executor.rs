//! This module provides types and constants for building an executor account resolver
//! https://github.com/wormholelabs-xyz/executor-account-resolver-svm/blob/main/modules/executor-account-resolver-svm/src/lib.rs

use anchor_lang::prelude::*;

pub const RESOLVER_RESULT_ACCOUNT_SEED: &[u8] = b"executor-account-resolver:result";
pub const RESOLVER_RESULT_ACCOUNT: &[u8; 8] = &[34, 185, 243, 199, 181, 255, 28, 227];
pub const RESOLVER_EXECUTE_VAA_V1: [u8; 8] = [148, 184, 169, 222, 207, 8, 154, 127];
pub const RESOLVER_PUBKEY_PAYER: Pubkey =
    Pubkey::new_from_array(*b"payer_00000000000000000000000000");
pub const RESOLVER_PUBKEY_SHIM_VAA_SIGS: Pubkey =
    Pubkey::new_from_array(*b"shim_vaa_sigs_000000000000000000");
pub const RESOLVER_PUBKEY_GUARDIAN_SET: Pubkey =
    Pubkey::new_from_array(*b"guardian_set_0000000000000000000");

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InstructionGroups(pub Vec<InstructionGroup>);

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InstructionGroup {
    pub instructions: Vec<SerializableInstruction>,
    pub address_lookup_tables: Vec<Pubkey>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SerializableInstruction {
    pub program_id: Pubkey,
    pub accounts: Vec<SerializableAccountMeta>,
    pub data: Vec<u8>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SerializableAccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<AccountMeta> for SerializableAccountMeta {
    fn from(account_meta: AccountMeta) -> Self {
        SerializableAccountMeta {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum Resolver<T> {
    Resolved(T),
    Missing(MissingAccounts),
    Account(),
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MissingAccounts {
    pub accounts: Vec<Pubkey>,
    pub address_lookup_tables: Vec<Pubkey>,
}

pub fn find_account<'c, 'info>(
    accs: &'c [AccountInfo<'info>],
    pubkey: Pubkey,
) -> Option<&'c AccountInfo<'info>> {
    accs.iter().find(|acc_info| *acc_info.key == pubkey)
}

pub fn missing_account(pubkey: Pubkey) -> Resolver<InstructionGroups> {
    Resolver::Missing(MissingAccounts {
        accounts: vec![pubkey],
        address_lookup_tables: vec![],
    })
}
