use anchor_lang::{
    prelude::*, solana_program::entrypoint::MAX_PERMITTED_DATA_INCREASE, system_program,
    InstructionData,
};
use anchor_spl::{
    associated_token::{
        self, get_associated_token_address_with_program_id,
        spl_associated_token_account::solana_program::system_instruction::MAX_PERMITTED_DATA_LENGTH,
    },
    token_2022,
};
use common::pda;
use executor_account_resolver_svm::{
    find_account, InstructionGroup, InstructionGroups, MissingAccounts, Resolver,
    SerializableAccountMeta, SerializableInstruction, RESOLVER_PUBKEY_PAYER,
    RESOLVER_PUBKEY_SHIM_VAA_SIGS, RESOLVER_RESULT_ACCOUNT, RESOLVER_RESULT_ACCOUNT_SEED,
};

use crate::{
    consts::AUTHORITY_SEED,
    errors::WormholeError,
    instruction::ReceiveMessage,
    instructions::{
        earn::{self, accounts::EarnGlobal},
        ext_swap::{self, accounts::SwapGlobal, types::WhitelistedExtension},
        order_book::{self, accounts::NativeOrder},
        portal, wormhole_verify_vaa_shim, VaaBody,
    },
    state::GLOBAL_SEED,
};

#[derive(Accounts)]
pub struct ResolveExecuteVaa {}

#[account(discriminator = RESOLVER_RESULT_ACCOUNT)]
pub struct ExecutorAccountResolverResult(Resolver<InstructionGroups>);

impl ResolveExecuteVaa {
    pub fn handler<'info>(
        ctx: Context<'_, '_, 'info, 'info, ResolveExecuteVaa>,
        vaa_body: Vec<u8>,
    ) -> Result<Resolver<InstructionGroups>> {
        let vaa = VaaBody::from_bytes(&vaa_body)?;

        // Accounts we need read data on
        let swap_global = pda!(&[GLOBAL_SEED], &ext_swap::ID);
        let earn_global = pda!(&[GLOBAL_SEED], &earn::ID);
        let result_account = pda!(&[RESOLVER_RESULT_ACCOUNT_SEED], &crate::ID);

        // Check for missing accounts
        {
            let mut missing = Vec::new();

            for account in [
                swap_global,
                earn_global,
                result_account,
                RESOLVER_PUBKEY_SHIM_VAA_SIGS,
            ] {
                if find_account(ctx.remaining_accounts, account).is_none() {
                    missing.push(account);
                }
            }

            if !missing.is_empty() {
                return Ok(Resolver::Missing(MissingAccounts {
                    accounts: missing,
                    address_lookup_tables: Vec::new(),
                }));
            }
        }

        // Increase the size of the return account then parse it
        let mut ret = {
            let return_account = find_account(ctx.remaining_accounts, result_account).unwrap();
            let system_account = find_account(ctx.remaining_accounts, System::id()).unwrap();

            // Find the payer account
            let payer_account = ctx
                .remaining_accounts
                .iter()
                .find(|acc_info| acc_info.is_signer && acc_info.is_writable)
                .ok_or(WormholeError::MissingPayerAccount)?;

            if !return_account.is_writable {
                return err!(WormholeError::InvalidReturnAccount);
            }

            let size = usize::min(
                return_account.data_len() + MAX_PERMITTED_DATA_INCREASE,
                MAX_PERMITTED_DATA_LENGTH.try_into()?,
            );

            let lamports = Rent::get()
                .unwrap()
                .minimum_balance(size)
                .saturating_sub(return_account.lamports());

            system_program::transfer(
                CpiContext::new(
                    system_account.to_account_info(),
                    system_program::Transfer {
                        from: payer_account.to_account_info(),
                        to: return_account.to_account_info(),
                    },
                ),
                lamports,
            )?;

            return_account.resize(size)?;

            Account::<ExecutorAccountResolverResult>::try_from(return_account)?
        };

        // Parse accounts we know are on remaining_accounts
        let swap_global_data =
            deserialize_account::<SwapGlobal>(ctx.remaining_accounts, swap_global)?;
        let earn_global_data =
            deserialize_account::<EarnGlobal>(ctx.remaining_accounts, earn_global)?;

        // Receive instruction for Portal
        let mut receive_message_ix = SerializableInstruction {
            program_id: crate::ID,
            data: ReceiveMessage {
                guardian_set_index: 0, // TODO
                vaa_body,
            }
            .data(),
            accounts: vec![
                SerializableAccountMeta {
                    pubkey: RESOLVER_PUBKEY_PAYER,
                    is_writable: true,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: pda!(&[GLOBAL_SEED], &crate::ID),
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: pda!(&[AUTHORITY_SEED], &crate::ID),
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: pda!(&[AUTHORITY_SEED], &portal::ID),
                    is_writable: false,
                    is_signer: false,
                },
                // SerializableAccountMeta {
                //     pubkey: guardian_set,
                //     is_writable: false,
                //     is_signer: false,
                // },
                // SerializableAccountMeta {
                //     pubkey: guardian_signatures,
                //     is_writable: false,
                //     is_signer: false,
                // },
                SerializableAccountMeta {
                    pubkey: pda!(&[GLOBAL_SEED], &earn::ID),
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: earn_global_data.m_mint,
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: wormhole_verify_vaa_shim::ID,
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: earn::ID,
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: token_2022::ID,
                    is_writable: false,
                    is_signer: false,
                },
            ],
        };

        // Add expected remaining accounts based on payload types
        match vaa.payload {
            common::Payload::TokenTransfer(token_transfer) => {
                if swap_global_data.whitelisted_extensions.is_empty() {
                    msg!("No whitelisted extensions");
                    return err!(WormholeError::InvalidSwapConfig);
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
                    &pda!(&[b"authority"], &portal::ID),
                    &earn_global_data.m_mint,
                    &token_2022::ID,
                );

                receive_message_ix.accounts.extend_from_slice(&[
                    SerializableAccountMeta {
                        pubkey: extension_mint,
                        is_writable: true,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: recipient_token_account,
                        is_writable: true,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: authority_m_token_account,
                        is_writable: false,
                        is_signer: true,
                    },
                    SerializableAccountMeta {
                        pubkey: extention_m_vault,
                        is_writable: true,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: extension_m_vault_auth,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: extension_mint_auth,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: extension_global,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: extension_token_program,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: extension_pid,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: swap_global,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: ext_swap::ID,
                        is_writable: false,
                        is_signer: false,
                    },
                ]);
            }
            common::Payload::FillReport(report) => {
                // PDAs
                let order = pda!(&[b"order", &report.order_id], &order_book::ID);
                let event_auth = pda!(&[b"__event_authority"], &order_book::ID);

                // Need order data to get mint
                if find_account(ctx.remaining_accounts, order).is_none() {
                    return Ok(Resolver::Missing(MissingAccounts {
                        accounts: vec![order],
                        address_lookup_tables: Vec::new(),
                    }));
                }

                let order_data = deserialize_account::<NativeOrder>(ctx.remaining_accounts, order)?;

                // Need mint account to see token program
                if find_account(ctx.remaining_accounts, order_data.token_in).is_none() {
                    return Ok(Resolver::Missing(MissingAccounts {
                        accounts: vec![order_data.token_in],
                        address_lookup_tables: Vec::new(),
                    }));
                }

                let token_in_program = find_account(ctx.remaining_accounts, order_data.token_in)
                    .unwrap()
                    .owner;

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

                receive_message_ix.accounts.extend_from_slice(&[
                    SerializableAccountMeta {
                        pubkey: RESOLVER_PUBKEY_PAYER,
                        is_writable: true,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: order,
                        is_writable: true,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: order_data.token_in,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: report.origin_recipient.into(),
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: recipient_token_account,
                        is_writable: true,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: order_token_account,
                        is_writable: true,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: *token_in_program,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: associated_token::ID,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: order_book::ID,
                        is_writable: false,
                        is_signer: false,
                    },
                    SerializableAccountMeta {
                        pubkey: event_auth,
                        is_writable: false,
                        is_signer: false,
                    },
                ]);
            }
            _ => {}
        }

        ret.set_inner(ExecutorAccountResolverResult(Resolver::Resolved(
            InstructionGroups(vec![InstructionGroup {
                instructions: vec![receive_message_ix],
                address_lookup_tables: vec![],
            }]),
        )));
        ret.exit(ctx.program_id)?;
        Ok(Resolver::Account())
    }
}

fn deserialize_account<T: AccountDeserialize>(
    remaining_accounts: &[AccountInfo],
    pubkey: Pubkey,
) -> Result<T> {
    let account = find_account(remaining_accounts, pubkey).unwrap();

    match T::try_deserialize(&mut &account.try_borrow_mut_data()?[..]) {
        Ok(data) => Ok(data),
        Err(e) => {
            msg!("Failed to deserialize account {}", pubkey);
            Err(e)
        }
    }
}
