use anchor_lang::prelude::*;
use anchor_lang::solana_program::program;
use anchor_spl::token_2022;
use m0_portal_common::{
    earn::{self, accounts::EarnGlobal},
    ext_swap::{self, accounts::SwapGlobal},
    pda,
    portal::{self, constants::MESSAGE_SEED},
    require_metas, Extension, Payload, AUTHORITY_SEED,
};

use crate::state::{LayerZeroGlobal, LzAccount, LzReceiveParams, GLOBAL_SEED};

#[derive(Accounts)]
pub struct LzReceiveTypes<'info> {
    #[account(
        seeds = [GLOBAL_SEED],
        bump = lz_global.bump,
    )]
    pub lz_global: Account<'info, LayerZeroGlobal>,
}

impl LzReceiveTypes<'_> {
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, LzReceiveTypes<'info>>,
        params: LzReceiveParams,
    ) -> Result<()> {
        let payload = Payload::decode(&params.message)?;

        let lz_global_key = ctx.accounts.lz_global.key();
        let endpoint_key = ctx.accounts.lz_global.endpoint_program;
        let lz_adapter_authority = pda!(&[AUTHORITY_SEED], &crate::ID);
        let portal_global = pda!(&[GLOBAL_SEED], &portal::ID);
        let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);
        let message_account = pda!(&[MESSAGE_SEED, &payload.header.message_id], &portal::ID);
        let earn_global_key = pda!(&[GLOBAL_SEED], &earn::ID);
        let swap_global_key = pda!(&[GLOBAL_SEED], &ext_swap::ID);

        // Read EarnGlobal from remaining_accounts to get m_mint
        let earn_global_data: EarnGlobal =
            deserialize_account(ctx.remaining_accounts, earn_global_key)?;
        let m_mint = earn_global_data.m_mint;

        // Read SwapGlobal from remaining_accounts to get whitelisted extensions
        let swap_global_data: SwapGlobal =
            deserialize_account(ctx.remaining_accounts, swap_global_key)?;
        let whitelisted_extensions: Vec<Extension> = swap_global_data
            .whitelisted_extensions
            .iter()
            .map(|&ext| Extension::from(ext))
            .collect();

        // For fill reports, try to find the token_in account to determine its token program
        let fill_report_token_in = match &payload.data {
            m0_portal_common::PayloadData::FillReport(report) => {
                let mint_key: Pubkey = report.token_in.into();
                ctx.remaining_accounts
                    .iter()
                    .find(|acc| *acc.key == mint_key)
            }
            _ => None,
        };

        // Resolve payload-specific remaining accounts
        let required_remaining = require_metas(
            &payload.data,
            m_mint,
            whitelisted_extensions,
            fill_report_token_in,
        )?;

        // Build the LZ endpoint clear PDAs
        let oapp_registry = pda!(&[b"OApp", lz_global_key.as_ref()], &endpoint_key);
        let nonce = Pubkey::find_program_address(
            &[
                b"Nonce",
                lz_global_key.as_ref(),
                &params.src_eid.to_be_bytes(),
                &params.sender,
            ],
            &endpoint_key,
        )
        .0;
        let payload_hash = Pubkey::find_program_address(
            &[
                b"PayloadHash",
                lz_global_key.as_ref(),
                &params.src_eid.to_be_bytes(),
                &params.sender,
                &params.nonce.to_be_bytes(),
            ],
            &endpoint_key,
        )
        .0;
        let endpoint_settings = pda!(&[b"Endpoint"], &endpoint_key);
        let event_authority = pda!(&[b"__event_authority"], &endpoint_key);

        // Named accounts for the lz_receive instruction
        let mut accounts: Vec<LzAccount> = vec![
            LzAccount {
                pubkey: Pubkey::default(), // payer — filled by executor
                is_signer: true,
                is_writable: true,
            },
            LzAccount {
                pubkey: lz_global_key,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: lz_adapter_authority,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: portal_global,
                is_signer: false,
                is_writable: true,
            },
            LzAccount {
                pubkey: portal_authority,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: message_account,
                is_signer: false,
                is_writable: true,
            },
            LzAccount {
                pubkey: earn_global_key,
                is_signer: false,
                is_writable: true,
            },
            LzAccount {
                pubkey: m_mint,
                is_signer: false,
                is_writable: true,
            },
            LzAccount {
                pubkey: token_2022::ID,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: earn::ID,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: portal::ID,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: anchor_lang::system_program::ID,
                is_signer: false,
                is_writable: false,
            },
            // remaining_accounts: clear accounts (CLEAR_ACCOUNTS_COUNT = 8)
            LzAccount {
                pubkey: oapp_registry,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: nonce,
                is_signer: false,
                is_writable: true,
            },
            LzAccount {
                pubkey: payload_hash,
                is_signer: false,
                is_writable: true,
            },
            LzAccount {
                pubkey: endpoint_settings,
                is_signer: false,
                is_writable: true,
            },
            LzAccount {
                pubkey: event_authority,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: endpoint_key,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: endpoint_key,
                is_signer: false,
                is_writable: false,
            },
            LzAccount {
                pubkey: endpoint_key,
                is_signer: false,
                is_writable: false,
            },
        ];

        // remaining_accounts: payload-specific accounts (after clear accounts)
        accounts.extend(required_remaining.into_iter().map(|meta| LzAccount {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        }));

        let bytes = accounts.try_to_vec()?;
        program::set_return_data(&bytes);

        Ok(())
    }
}

fn deserialize_account<T: AccountDeserialize>(
    remaining_accounts: &[AccountInfo],
    pubkey: Pubkey,
) -> Result<T> {
    let account = remaining_accounts
        .iter()
        .find(|acc| *acc.key == pubkey)
        .ok_or(BridgeError::MissingOptionalAccount)?;

    T::try_deserialize(&mut &account.try_borrow_data()?[..])
}

use m0_portal_common::BridgeError;
