use anchor_lang::{
    prelude::*, solana_program::entrypoint::MAX_PERMITTED_DATA_INCREASE, system_program,
    InstructionData,
};
use anchor_spl::{
    associated_token::{
        spl_associated_token_account::solana_program::system_instruction::MAX_PERMITTED_DATA_LENGTH,
    }
};
use common::{
    BridgeError, earn::{self, accounts::EarnGlobal}, ext_swap::{self, accounts::SwapGlobal}, order_book::{self, accounts::NativeOrder, constants::ORDER_SEED_PREFIX}, pda, portal, require_metas, wormhole_verify_vaa_shim
};
use executor_account_resolver_svm::{
    find_account, InstructionGroup, InstructionGroups, MissingAccounts, Resolver,
    SerializableAccountMeta, SerializableInstruction, RESOLVER_PUBKEY_PAYER,
    RESOLVER_PUBKEY_SHIM_VAA_SIGS, RESOLVER_RESULT_ACCOUNT, RESOLVER_RESULT_ACCOUNT_SEED,
};

use crate::{
    consts::AUTHORITY_SEED, instruction::ReceiveMessage, instructions::VaaBody, state::GLOBAL_SEED,
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

        let result_account = pda!(&[RESOLVER_RESULT_ACCOUNT_SEED], &crate::ID);
        let mut earn_global_data: Option<EarnGlobal> = None;
        let mut swap_global_data: Option<SwapGlobal> = None;
        let mut order_data: Option<NativeOrder> = None;
        let mut order_token_in: Option<AccountInfo> = None;

        // Check for missing accounts
        {
            let mut accounts_required = vec![result_account, RESOLVER_PUBKEY_SHIM_VAA_SIGS];

            match vaa.payload {
                common::Payload::TokenTransfer(_) => {
                    let earn_global = pda!(&[GLOBAL_SEED], &earn::ID);
                    earn_global_data =
                        deserialize_account(ctx.remaining_accounts, earn_global).ok();

                    let swap_global = pda!(&[GLOBAL_SEED], &ext_swap::ID);
                    swap_global_data =
                        deserialize_account(ctx.remaining_accounts, swap_global).ok();

                    accounts_required.extend([earn_global, swap_global]);
                }
                common::Payload::FillReport(ref report) => {
                    // Need order data to get mint
                    let order = pda!(&[ORDER_SEED_PREFIX, &report.order_id], &order_book::ID);
                    order_data =
                        deserialize_account::<NativeOrder>(ctx.remaining_accounts, order).ok();

                    accounts_required.push(order);

                    if let Some(ref o) = order_data {
                        // Need mint account to see token program
                        order_token_in = find_account(ctx.remaining_accounts, o.token_in).cloned();
                        accounts_required.push(o.token_in);
                    }
                }
                _ => {}
            }

            let missing: Vec<_> = accounts_required
                .into_iter()
                .filter(|&account| find_account(ctx.remaining_accounts, account).is_none())
                .collect();

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
                .ok_or(BridgeError::MissingPayerAccount)?;

            if !return_account.is_writable {
                return err!(BridgeError::InvalidReturnAccount);
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

        // Receive instruction on Wormhole adapter
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
                    pubkey: wormhole_verify_vaa_shim::ID,
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: portal::ID,
                    is_writable: false,
                    is_signer: false,
                },
                SerializableAccountMeta {
                    pubkey: system_program::ID,
                    is_writable: false,
                    is_signer: false,
                },
            ],
        };

        let required_remaining = require_metas(
            &vaa.payload,
            RESOLVER_PUBKEY_PAYER,
            swap_global_data,
            earn_global_data,
            order_data,
            order_token_in,
        )?;

        // Add expected remaining accounts based on payload type
        receive_message_ix
            .accounts
            .extend(required_remaining.iter().cloned().map(|a| a.into()));

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
    let account =
        find_account(remaining_accounts, pubkey).ok_or(BridgeError::MissingOptionalAccount)?;

    match T::try_deserialize(&mut &account.try_borrow_mut_data()?[..]) {
        Ok(data) => Ok(data),
        Err(e) => {
            msg!("Failed to deserialize account {}", pubkey);
            Err(e)
        }
    }
}
